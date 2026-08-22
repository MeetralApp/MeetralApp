use super::{
    config_from_state, unix_ms_now_u64, watchdog_should_scan_devices, watchdog_tick_sleep_ms,
    BridgeConnectionState, Direction, SharedEngine, TranslationEngine,
    ENSURE_DIRECT_AUDIO_EVERY_TICKS,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::audio::{list_devices_async, AudioFaultEvent};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn is_watchdog_degraded(engine: &TranslationEngine) -> bool {
    engine.ensure_direct_audio_pending
        || engine.outbound_side.audio_path.is_degraded()
        || engine.inbound_side.audio_path.is_degraded()
        || engine.outbound_side.bridge == BridgeConnectionState::Reconnecting
        || engine.inbound_side.bridge == BridgeConnectionState::Reconnecting
        || engine.outbound_side.reconnect_attempt.is_some()
        || engine.inbound_side.reconnect_attempt.is_some()
}

impl TranslationEngine {
    pub(crate) fn ensure_watchdog(&mut self, app: AppHandle, engine: SharedEngine) {
        self.ensure_audio_fault_listeners(engine.clone(), app.clone());
        if self.watchdog_cancel.is_some() {
            return;
        }
        let cancel = CancellationToken::new();
        let child = cancel.child_token();
        self.watchdog_cancel = Some(cancel);

        // At most one health check in flight: a slow device open must not pile
        // up spawned tasks that each contend on the engine lock.
        let health_check_in_flight = Arc::new(AtomicBool::new(false));

        // OS-level endpoint notifications wake the loop instantly; the polling
        // cadence below stays as the fallback path.
        let (device_event_tx, device_event_rx) = mpsc::unbounded_channel();
        let mut device_event_rx = Some(device_event_rx);
        let device_monitor = crate::audio::start_device_change_monitor(device_event_tx);
        if device_monitor.is_none() {
            tracing::warn!("audio device change monitor unavailable; polling only");
        }

        tokio::spawn(async move {
            // Held for the lifetime of the loop; dropping unregisters the OS listener.
            let _device_monitor = device_monitor;
            while !child.is_cancelled() {
                let (degraded, tick_count, cached_devices) = {
                    let guard = engine.lock().await;
                    (
                        is_watchdog_degraded(&guard),
                        guard.watchdog_tick_count,
                        guard
                            .cached_device_catalog
                            .as_ref()
                            .map(|catalog| catalog.devices.clone()),
                    )
                };

                let woke_by_event = tokio::select! {
                                   _ = child.cancelled() => break,
                                   _ = tokio::time::sleep(std::time::Duration::from_millis(
                                       watchdog_tick_sleep_ms(degraded),
                                   )) => false,
                                   received = async {
                                       match device_event_rx.as_mut() {
                                           Some(rx) => rx.recv().await,
                                           None => std::future::pending().await,
                                       }
                                   } => {
                                       if received.is_none() {
                // Monitor ended (sender dropped) — stop selecting on it.
                                           device_event_rx = None;
                                           false
                                       } else {
                // Events arrive in bursts during endpoint churn;
                // drain the coalesced remainder of the burst.
                                           if let Some(rx) = device_event_rx.as_mut() {
                                               while rx.try_recv().is_ok() {}
                                           }
                                           true
                                       }
                                   }
                               };

                let should_scan = woke_by_event
                    || watchdog_should_scan_devices(degraded, tick_count.saturating_add(1));

                let config = config_from_state(&app).await;
                let devices = if should_scan {
                    list_devices_async().await
                } else if let Some(devices) = cached_devices {
                    Ok(devices)
                } else {
                    list_devices_async().await
                };

                // Update phase: hold the engine lock only for the in-memory state
                // transition, passing the already-gathered config and devices.
                if let Ok(ref devices) = devices {
                    let should_ensure_direct = {
                        let mut guard = engine.lock().await;
                        let now_ms = unix_ms_now_u64();
                        guard.update_publish_context(&config, devices, now_ms);
                        guard.try_recover_lost_audio_paths(&config, devices);
                        let devices_changed = guard
                            .cached_device_catalog
                            .as_ref()
                            .map(|catalog| {
                                crate::app_state::devices_list_changed(&catalog.devices, devices)
                            })
                            .unwrap_or(true);
                        if devices_changed {
                            guard.ensure_direct_audio_pending = true;
                            guard.note_catalog_change(now_ms);
                            tracing::debug!("device catalog changed; acting once it stays stable");
                        }
                        guard.publish(&app);
                        // Ensure/recovery only runs against a settled catalog —
                        // never on a mid-churn enumeration.
                        guard.catalog_stable(now_ms)
                            && (guard.ensure_direct_audio_pending
                                || guard
                                    .watchdog_tick_count
                                    .is_multiple_of(ENSURE_DIRECT_AUDIO_EVERY_TICKS))
                    };

                    if !health_check_in_flight.swap(true, Ordering::SeqCst) {
                        let engine_health = engine.clone();
                        let app_health = app.clone();
                        let config_health = config.clone();
                        let devices_health = devices.clone();
                        let in_flight = health_check_in_flight.clone();
                        tokio::spawn(async move {
                            super::lock_scope::check_audio_health_shared(
                                &engine_health,
                                &app_health,
                                &config_health,
                                &devices_health,
                            )
                            .await;
                            in_flight.store(false, Ordering::SeqCst);
                        });
                    }

                    if should_ensure_direct {
                        super::lock_scope::maybe_ensure_direct_audio_shared(
                            &engine, &config, devices, &app,
                        )
                        .await;
                        let mut guard = engine.lock().await;
                        guard.ensure_direct_audio_pending = false;
                    }
                    super::lock_scope::check_proactive_session_refresh_shared(
                        &engine, &app, &config,
                    )
                    .await;
                    super::lock_scope::check_meeting_idle_end_shared(&engine, &app, &config).await;
                } else {
                    super::lock_scope::check_proactive_session_refresh_shared(
                        &engine, &app, &config,
                    )
                    .await;
                    super::lock_scope::check_meeting_idle_end_shared(&engine, &app, &config).await;
                }
            }
        });
    }

    pub(super) fn ensure_audio_fault_listeners(&mut self, engine: SharedEngine, app: AppHandle) {
        if self.audio_fault_listeners_started {
            return;
        }
        self.audio_fault_listeners_started = true;

        let (out_tx, out_rx) =
            mpsc::channel(crate::runtime::control_channel::AUDIO_FAULT_CHANNEL_DEPTH);
        let (in_tx, in_rx) =
            mpsc::channel(crate::runtime::control_channel::AUDIO_FAULT_CHANNEL_DEPTH);
        self.outbound_side.audio_fault_tx = Some(out_tx);
        self.inbound_side.audio_fault_tx = Some(in_tx);

        Self::spawn_audio_fault_listener(
            Direction::Outbound,
            engine.clone(),
            app.clone(),
            out_rx,
            self.listener_shutdown.child_token(),
        );
        Self::spawn_audio_fault_listener(
            Direction::Inbound,
            engine,
            app,
            in_rx,
            self.listener_shutdown.child_token(),
        );
    }

    pub(super) fn spawn_audio_fault_listener(
        direction: Direction,
        engine: SharedEngine,
        app: AppHandle,
        mut fault_rx: mpsc::Receiver<AudioFaultEvent>,
        shutdown: CancellationToken,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    msg = fault_rx.recv() => {
                        let Some(_fault) = msg else { break };
                        let config = config_from_state(&app).await;
                        let Ok(devices) = list_devices_async().await else {
                            continue;
                        };
                        super::lock_scope::handle_audio_fault_shared(
                            direction, &engine, &config, &devices, &app,
                        )
                        .await;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::helpers::{
        watchdog_should_scan_devices, watchdog_tick_sleep_ms, MEETING_IDLE_NO_SEGMENT_MS,
        WATCHDOG_DEVICE_SCAN_EVERY_TICKS_IDLE, WATCHDOG_TICK_IDLE_MS, WATCHDOG_TICK_STABLE_MS,
    };
    use crate::meeting::{
        clear_meeting_idle_clock, meeting_idle_elapsed_ms, new_meeting_idle_clock,
    };

    #[test]
    fn watchdog_tick_sleep_ms_adapts_to_health() {
        assert_eq!(watchdog_tick_sleep_ms(true), WATCHDOG_TICK_STABLE_MS);
        assert_eq!(watchdog_tick_sleep_ms(false), WATCHDOG_TICK_IDLE_MS);
    }

    #[test]
    fn watchdog_scans_devices_every_tick_as_notification_fallback() {
        assert!(watchdog_should_scan_devices(true, 1));
        assert!(watchdog_should_scan_devices(false, 1));
        assert!(watchdog_should_scan_devices(
            false,
            WATCHDOG_DEVICE_SCAN_EVERY_TICKS_IDLE,
        ));
    }

    #[test]
    fn meeting_idle_threshold_triggers_after_five_minutes() {
        let clock = new_meeting_idle_clock();
        assert_eq!(meeting_idle_elapsed_ms(&clock, 10_000), None);

        clock.store(1_000, std::sync::atomic::Ordering::Relaxed);
        assert!(
            meeting_idle_elapsed_ms(&clock, 1_000 + MEETING_IDLE_NO_SEGMENT_MS - 1).unwrap_or(0)
                < MEETING_IDLE_NO_SEGMENT_MS
        );
        assert_eq!(
            meeting_idle_elapsed_ms(&clock, 1_000 + MEETING_IDLE_NO_SEGMENT_MS),
            Some(MEETING_IDLE_NO_SEGMENT_MS)
        );
    }

    #[test]
    fn meeting_idle_timer_resets_on_touch_and_clears() {
        let clock = new_meeting_idle_clock();
        clock.store(1_000, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            meeting_idle_elapsed_ms(&clock, 1_000 + MEETING_IDLE_NO_SEGMENT_MS),
            Some(MEETING_IDLE_NO_SEGMENT_MS)
        );

        clock.store(50_000, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(meeting_idle_elapsed_ms(&clock, 50_000 + 1_000), Some(1_000));

        clear_meeting_idle_clock(&clock);
        assert_eq!(meeting_idle_elapsed_ms(&clock, 100_000), None);
    }
}
