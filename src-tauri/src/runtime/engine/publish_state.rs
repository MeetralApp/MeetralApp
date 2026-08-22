use tauri::{AppHandle, Emitter};

use crate::app_state::{
    build_app_status, build_column_ui_snapshot, build_setup_state, devices_list_changed,
    snapshot_semantically_eq, AppSnapshot, ConfigState, DeviceCatalogState, EngineView,
    PublishContext,
};
use crate::audio::AudioDeviceInfo;
use crate::config::{AppConfig, ConfigView};

use super::TranslationEngine;

impl TranslationEngine {
    pub fn update_publish_context(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        now_ms: u64,
    ) {
        self.publish_ctx = PublishContext::new(config.clone(), devices.to_vec(), now_ms);
    }

    pub fn publish_context(&self) -> &PublishContext {
        &self.publish_ctx
    }

    fn build_engine_view(&self) -> EngineView<'_> {
        EngineView {
            config: &self.publish_ctx.config,
            devices: &self.publish_ctx.devices,
            outbound_pipeline: self.status.0.clone(),
            inbound_pipeline: self.status.1.clone(),
            outbound_audio_raw: self.outbound_side.audio_path.connection_state(),
            inbound_audio_raw: self.inbound_side.audio_path.connection_state(),
            outbound_audio_attempt: self.outbound_side.audio_path.reconnect_attempt(),
            inbound_audio_attempt: self.inbound_side.audio_path.reconnect_attempt(),
            runtime_error: self.last_error.clone(),
            outbound_active_since: self.outbound_side.active_since,
            inbound_active_since: self.inbound_side.active_since,
            outbound_bridge: self.outbound_side.bridge.clone(),
            inbound_bridge: self.inbound_side.bridge.clone(),
            outbound_bridge_attempt: self.outbound_side.reconnect_attempt,
            inbound_bridge_attempt: self.inbound_side.reconnect_attempt,
            running: self.is_busy(),
            mic_muted: self.mic_mute.is_muted(),
            speaker_muted: self.speaker_mute.is_muted(),
        }
    }

    fn sync_device_catalog(&mut self, devices: &[AudioDeviceInfo], now_ms: u64) {
        let previous = self
            .cached_device_catalog
            .as_ref()
            .map(|catalog| catalog.devices.as_slice());
        if previous
            .map(|prev| devices_list_changed(prev, devices))
            .unwrap_or(true)
        {
            self.devices_revision = self.devices_revision.saturating_add(1);
            self.cached_device_catalog = Some(DeviceCatalogState {
                revision: self.devices_revision,
                devices: devices.to_vec(),
                enumerated_at: now_ms,
            });
        }
    }

    pub fn build_app_snapshot(&self) -> AppSnapshot {
        let ctx = &self.publish_ctx;
        let view = self.build_engine_view();
        let setup = build_setup_state(&view, self.app_state_revision.max(1));
        let columns = build_column_ui_snapshot(&view, &setup);
        let runtime = build_app_status(&view);
        let devices = self
            .cached_device_catalog
            .clone()
            .unwrap_or(DeviceCatalogState {
                revision: self.devices_revision.max(1),
                devices: ctx.devices.clone(),
                enumerated_at: ctx.now_ms,
            });
        let config = ConfigState {
            revision: self.config_revision.max(1),
            config: ConfigView::from(&ctx.config),
        };
        AppSnapshot {
            revision: self.app_state_revision.max(1),
            runtime,
            setup,
            columns,
            devices,
            config,
        }
    }

    /// Single publish entry point — always emits a consistent `app-state` snapshot.
    pub fn publish(&mut self, app: &AppHandle) {
        let ctx = self.publish_ctx.clone();
        self.sync_device_catalog(&ctx.devices, ctx.now_ms);

        let next = self.build_app_snapshot();
        let changed = self
            .last_published_snapshot
            .as_ref()
            .map(|previous| !snapshot_semantically_eq(previous, &next))
            .unwrap_or(true);

        if changed {
            self.app_state_revision = self.app_state_revision.saturating_add(1);
            let mut emitted = self.build_app_snapshot();
            emitted.revision = self.app_state_revision;
            self.last_published_snapshot = Some(emitted.clone());
            let _ = app.emit("app-state", &emitted);
            crate::tray::update_tray_ui(app, &emitted.runtime);
            sync_overlay_visibility(app, &emitted.runtime);
        }
    }

    pub fn publish_with_context(
        &mut self,
        app: &AppHandle,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        now_ms: u64,
    ) {
        self.update_publish_context(config, devices, now_ms);
        self.publish(app);
    }

    pub fn status_snapshot(&self) -> crate::runtime::engine::AppStatus {
        build_app_status(&self.build_engine_view())
    }

    pub fn seed_publish_context(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        now_ms: u64,
    ) {
        self.update_publish_context(config, devices, now_ms);
        self.sync_device_catalog(devices, now_ms);
        if self.config_revision == 0 {
            self.config_revision = 1;
        }
        if self.app_state_revision == 0 {
            self.app_state_revision = 1;
        }
        let snapshot = self.build_app_snapshot();
        self.last_published_snapshot = Some(snapshot);
    }

    pub fn bump_config_revision(&mut self, config: &AppConfig) {
        self.config_revision = self.config_revision.saturating_add(1);
        self.publish_ctx.config = config.clone();
    }

    pub fn apply_inbound_ducking_from_config(&self, config: &AppConfig) {
        self.inbound.apply_ducking_params_from_config(config);
    }

    /// Preview ducked-original settings on the live inbound mixer without touching AppConfig/disk.
    pub fn preview_inbound_ducking(&self, enabled: bool, gain: f32) {
        self.inbound.apply_ducking_params(enabled, gain);
    }
}

fn sync_overlay_visibility(app: &AppHandle, status: &crate::runtime::engine::AppStatus) {
    let session_active = matches!(
        status.outbound,
        crate::runtime::engine::PipelineState::Starting
            | crate::runtime::engine::PipelineState::Stopping
            | crate::runtime::engine::PipelineState::Active
    ) || matches!(
        status.inbound,
        crate::runtime::engine::PipelineState::Starting
            | crate::runtime::engine::PipelineState::Stopping
            | crate::runtime::engine::PipelineState::Active
    );
    crate::overlay::on_pipeline_status(app, session_active);
}
