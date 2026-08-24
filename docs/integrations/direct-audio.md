# Direct audio passthrough

**Audience:** anyone changing Direct standby relay, `keep_direct_audio`, or Direct idle CPU / latency (Windows + macOS).  
**Related:** [macos.md](../development/macos.md), [catalog.md](../features/catalog.md) Direct row.

Read this before editing Direct relay, `keep_direct_audio`, or standby CPU.

---

## Agent summary

| Item | Fact |
|------|------|
| **Direct** | Local passthrough (mic → Teams feed; meeting capture → headphones). **No** live API. |
| **Translate** | Capture → Gemini **or** OpenAI **or** Soniox → TTS playback. Direct relay **stops** for the direction that is translating. |
| **Original under translation** | Meeting → You Translated/Custom may mix a ducked floor under TTS inside `spawn_pipeline_audio` — **not** via `DirectRelay`. Use headphones; do not route Local Playback into Meeting Capture. |
| **Config** | `keep_direct_audio` (default `true`) — Settings → Audio → Routing (Save) **or** tray “Keep Direct audio”. |
| **Entry** | `ensure_direct_audio_shared` → `DirectRelay::start_outbound/inbound` |
| **Types** | `PipelineState` in `runtime/engine/types.rs` (`Direct` \| `Active` \| `Off` \| …) |
| **Platform** | Windows + macOS: **single-thread (or 1 management thread) passthrough**. Other OS: legacy multi-thread (not optimized). |

### Do not break

1. **Do not** restore spin/poll render at WASAPI buffer frequency when the queue is empty — that caused ~15% idle CPU.
2. **Do not** split capture + playback + tokio relay again on Win/macOS Direct unless you re-measure CPU.
3. Keep `DIRECT_FRAME_MS = 20` for passthrough. `FRAME_MS = 100` (`config/mod.rs`) is the Translate upload chunk, not Direct.
4. Always call `stop_standby_direct_audio` when `!keep_direct_audio`.
5. Playback may pad silence for **one buffer** on a real callback/period — that is not a busy-loop silence fill.

---

## Product flow

```
App ready / tray show / device-change event / watchdog tick
    → ensure_direct_audio (if keep_direct_audio && !busy translating)
        → DirectRelay::start_outbound  (UserMic → TeamsMicFeed)
        → DirectRelay::start_inbound   (MeetingCapture → LocalPlayback)

User enables Translate on one column
    → stop Direct relay for that direction
    → OutboundPipeline / InboundPipeline (Gemini | OpenAI | Soniox)

User disables Translate
    → resume_direct_*_if_enabled (if keep_direct_audio)
```

---

## File map

| File | Role |
|------|------|
| `src-tauri/src/runtime/direct_relay.rs` | `DirectRelay`; `start_direct_relay` → platform passthrough |
| `src-tauri/src/runtime/engine/direct_audio.rs` | start/stop Direct on the engine |
| `src-tauri/src/runtime/engine/lock_scope.rs` | `ensure_direct_audio_shared`, `stop_standby_direct_audio` |
| `src-tauri/src/runtime/engine/watchdog.rs` | Health check, ensure when catalog is stable, `select!` tick vs device-change, stop when setting off |
| `src-tauri/src/audio/device_monitor.rs` | Payload-free `DeviceChangeEvent` + platform monitor (wakes watchdog) |
| `src-tauri/src/audio/backend/windows/endpoint_notify.rs` | Windows `IMMNotificationClient` on a COM thread |
| `src-tauri/src/audio/backend/macos/listener.rs` | macOS `DeviceListListener` |
| `src-tauri/src/app_state/sync.rs` | `sync_app_state_after_config_change` — stop relay on Save when off |
| `src-tauri/src/audio/mod.rs` | Re-exports `start_direct_passthrough` (Win + macOS) |
| `src-tauri/src/audio/backend/windows/direct_passthrough.rs` | Windows implementation |
| `src-tauri/src/audio/backend/macos/direct_passthrough.rs` | macOS implementation |

Translate pipeline (unchanged by Direct): `pipeline/outbound/`, `pipeline/inbound/`, `audio/runtime.rs` (`spawn_pipeline_audio`).

There is **no** `pipeline/direct.rs`.

---

## Windows — `direct_passthrough.rs`

- **1 thread** per direction (`direct-outbound`, `direct-inbound`).
- Same thread opens WASAPI capture + render; loop **interleaves**:
  1. `drain_render_pending` — write render from `get_available_space_in_frames()` (~device period).
  2. Read capture → when a chunk is full → convert → `render_pending`.
  3. Sleep/event: prefer render wait if pending; paced idle only when both idle.

```rust
const DIRECT_FRAME_MS: u32 = 20;              // passthrough only
const MAX_RENDER_PENDING_FRAMES: usize = 4;     // ~80 ms backlog cap
```

Wait scheduling: if work happened, wait render (if pending) else wait capture; if idle with render pending, wait render (do not paced-wait capture); if both empty, `wait_for_audio_paced(20ms)`.

`trim_render_backlog` drops oldest frames when pending exceeds `4 × render_frame_bytes`.

Translate playback (`windows/playback.rs`) **always** pads silence to keep the buffer full. Direct **does not** spin when idle; it only pads the **tail of one buffer** while draining a real frame.

---

## macOS — `direct_passthrough.rs`

Core Audio is callback-driven (unlike Windows poll):

- **1 management thread** per direction: register IO procs, sleep 20 ms until stop (+ poll sample rate ~100 ms).
- **2 IO procs** on 2 HAL devices: `capture_io_proc` → `PassthroughClient::ingest_capture`; `playback_io_proc` → `fill_playback_output`.
- Shared state: `Mutex<PassthroughClient>`. IO procs use `try_lock`; playback falls back to `write_silence_output` on lock fail.
- **Cold-start:** `AudioDeviceStart` capture first → settle rates (~250 ms) → `reconfigure_rates` → start playback. `SampleRateListener` on both devices.

Same semantics as Windows: `DIRECT_FRAME_MS = 20`, `MAX_RENDER_PENDING_FRAMES = 4`, resample device → 48 kHz → playback, `gate_pcm_in_place`, `CaptureHeartbeat::touch`, `DeviceAliveListener`, open retry ×6.

Direct on macOS does **not** go through `start_user_mic_capture` / tokio forward / `spawn_pipeline_audio`.

---

## Engine lifecycle

When `keep_direct_audio = false`, `stop_standby_direct_audio` runs from ensure, watchdog/health, and `sync_app_state_after_config_change` (Save).

Health: `DirectRelay::is_outbound_healthy` / `is_inbound_healthy` — capture device still matches and playback role resolves. `CaptureHeartbeat` touches every passthrough frame (~20 ms). Stale: `AUDIO_HEARTBEAT_STALE_MS = 3000` (`runtime/engine/helpers.rs`) → watchdog restart. Faults: `AudioFaultEvent` → `engine/recovery.rs`.

### Device churn

Bluetooth HFP / virtual-mixer engine restarts can churn the endpoint list for a few seconds. Current defenses:

- **Catalog stability window** — `AUDIO_CATALOG_STABLE_MS = 3000`; ensure/recovery only after 3 s with no catalog diff.
- **Resolve-before-kill** — validate target devices before tearing down a healthy relay.
- **Startup grace** — skip Recover for `AUDIO_STARTUP_GRACE_MS` (8 s) after relay start.
- **Fault deferral** — if catalog is unstable, drop an already-dead worker and restart at settle via ensure.
- **Detection** — OS notifications wake watchdog via `select!` (payload-free “re-enumerate”). Polling is fallback (idle scan each ~8 s tick).

Trade-off: a **real** unplug recovers ~3 s slower; false disconnects during churn stop.

`CancellationToken` is passed into `DirectRelay` but **unused** on Win/macOS passthrough — stop is `CaptureHandle::stop()` (`AtomicBool` + join).

---

## Platform matrix

| Platform | Implementation | Threads / direction (Direct) |
|----------|----------------|------------------------------|
| Windows | `windows/direct_passthrough.rs` | 1 |
| macOS | `macos/direct_passthrough.rs` | 1 (+ Core Audio callbacks) |
| Other | `runtime/direct_relay.rs` legacy tokio path | 3+ |

```rust
#[cfg(any(windows, target_os = "macos"))]
// → start_direct_passthrough(...)

#[cfg(not(any(windows, target_os = "macos")))]
// → legacy capture + tokio + spawn_pipeline_audio
```

---

## QA (manual)

| # | Scenario | Expected |
|---|----------|----------|
| 1 | Direct both columns, silence 5 min | Low CPU (~1% or Translate-idle equivalent) |
| 2 | Direct, talk 10 min | No mic/speaker stutter |
| 3 | `keep_direct_audio` OFF + Save | CPU drops; relay fully stopped |
| 4 | Translate one column | Other column Direct if setting on |
| 5 | Direct → Translate → Direct | Relay resumes; no thread leak |
| 6 | Change audio device on Direct | Watchdog restart or healthy |
| 7 | Mute mic / speaker | Passthrough gated, no crash |
| 8 | Bluetooth HFP rate switch | Playback follows; no stretched audio |
| 9 | Cold launch Keep Direct | Log `settled` + correct rates after settle |

Idle Direct on 2ch devices should stay well under Translate residual CPU. Prefer BlackHole **2ch** over 16ch for lower callback CPU when choosing a Teams mic-feed device.

Same-rate Direct uses a **float32** path (skip f32↔i16); rate mismatch keeps i16 resample. Log includes `path=float32|i16-resample`.

---

## Debug

| Symptom | Check |
|---------|--------|
| High Direct CPU | New spin loop? extra thread? `keep_direct_audio` off but relay still running? |
| Stutter | `render_pending` drained fast enough? frame too large? render underrun |
| High latency | `MAX_RENDER_PENDING_FRAMES`, `DIRECT_FRAME_MS` |
| Watchdog restart loop | Heartbeat not touching (dead capture device?) |
| Restart storm after BT/HFP | Catalog stable for `AUDIO_CATALOG_STABLE_MS` before ensure? notify thread alive? |
| Relay killed while device still present | Resolve-before-kill must defer — recovery defer logs |

Log strings: `direct passthrough: ... → ... (20 ms frames, ...)`, `dropped N stale render frame(s)`, `direct outbound/inbound relay started`.

**Do not** fold `DIRECT_FRAME_MS` into global `FRAME_MS` (breaks Translate upload chunking). **Do not** introduce a shared cross-platform Direct trait unless CI covers both Win and macOS — poll vs callback models differ.
