# Architecture — module boundaries

**Status:** Meetral `4.8.6` production baseline — live translation + meeting library + summaries.  
**Product:** [features/catalog.md](../features/catalog.md) · **Now:** [features/current.md](../features/current.md) · **Router:** [AGENTS.md](../../AGENTS.md)

Read this before changing `src-tauri/src/**` or `src/features/**`.

---

## Goals

1. **Capability-driven routing** — pipeline, session, mux, and engine branch on `ProviderCapabilities` / `PlaybackSource`, not `AiProvider` identity.
2. **Vertical provider slices** — each vendor lives under `providers/<name>/` (Soniox STT+TTS co-located).
3. **`match AiProvider` only on the Hard-rules allow-list** (not in pipeline / session / mux / engine).
4. **One crate** — no Cargo workspace split.

---

## Module map (Rust)

```
src-tauri/src/
├── capabilities/        # catalog + ProviderCapabilities + PlaybackSource + routing helpers
├── providers/
│   ├── gemini/          # live + summary (GeminiChatLlm impl ChatLlmProvider)
│   ├── openai/          # live + summary (OpenAiChatLlm impl ChatLlmProvider)
│   ├── compatible/      # OpenAI-compatible custom profiles (chat) — one slice for local servers
│   ├── soniox/          # live (STT) + tts + context
│   ├── elevenlabs/      # TTS custom voice (outbound + inbound)
│   ├── fishaudio/       # TTS custom voice (outbound + inbound) — not a live AiProvider
│   ├── xai/             # TTS custom voice (outbound + inbound) — not a live AiProvider
│   └── shared/live/     # provider-neutral live bridge kernel
├── runtime/
│   ├── engine/          # TranslationEngine orchestrator
│   ├── factories/       # AiProvider / voice-output matches
│   ├── voice_runtime.rs
│   ├── playback_mux.rs
│   └── direct_relay.rs
├── pipeline/            # inbound / outbound session wiring
├── audio/               # platform capture/playback + device_monitor
├── meeting/             # SQLite library + SegmentEngine + summary + recording
├── overlay/             # transcript overlay window (Win hide-from-capture; Mac best-effort)
├── config/              # AppConfig (+ vendor sub-structs; ConfigView maps 1:1)
├── config_store/        # persistence + migration
├── app_data.rs          # durable AppData root (Win: unvirtualized Roaming + \\?\ I/O)
├── ai/                  # AiProvider enum, live handle, summary client (thin re-exports — do not grow)
│   └── llm/             # ChatLlmProvider trait + ChatRequest + LlmError/LlmErrorKind
├── voice/               # shared TTS types + config (thin re-exports — do not grow)
└── commands/            # thin Tauri IPC
```

| Concern | Path |
|---------|------|
| Live + TTS providers | `providers/{gemini,openai,soniox,elevenlabs,fishaudio,xai}/` |
| Shared live bridge kernel | `providers/shared/live/` |
| Caps / catalog / routing | `capabilities/{catalog,tts}.rs` |
| Factories (`match AiProvider` / custom voice vendor) | `runtime/factories/{live,setup,voice,summary,custom}.rs` |
| Voice runtime / mux / direct | `runtime/{voice_runtime,playback_mux,direct_relay}.rs` |
| Session wiring | `pipeline/{inbound,outbound}/` |
| Engine orchestrator | `runtime/engine/` |
| Overlay | `overlay/` + `src/features/overlay/` |
| Config domain / persistence | `config/` + `config_store/` — vendor settings in `SonioxSettings` / `ElevenLabsSettings` / `FishAudioSettings` / `XaiSettings` (flatten serde); `ConfigView` maps 1:1 camelCase |
| Frontend features | `src/features/{pipeline,config,ai,voice,audio,meeting,overlay}/` |
| Frontend shared | `src/shared/{ui,components,layout,hooks,context,lib}/` |
| Pipeline IPC | `src/features/pipeline/api/pipelineApi.ts` |

Rust `ai/` and `voice/` are thin facades. New Rust code imports `capabilities` / `providers` directly.

Frontend `src/features/ai/` and `src/features/voice/` are real UI features (catalog, keys, TTS). They are not facades.

Related: [pipeline.md](pipeline.md), [capabilities.md](capabilities.md), [../integrations/providers.md](../integrations/providers.md).

---

## Hard rules

1. **`match AiProvider` / `== AiProvider::Soniox`** only in:
   - `providers/**`
   - `runtime/factories/**`
   - `capabilities/catalog.rs` (catalog construction)
   - `config/**` migrate/normalize/tests
   - `config_store/**` persist/read keys
   - `commands/config.rs` IPC defaults
   - `meeting/prompts/**` (summary language lists via catalog helpers)
2. **Session / mux / engine** use `ProviderCapabilities`, `PlaybackSource`, and factory helpers — never provider identity.
3. **New live provider** = new folder under `providers/` + one registration in factories — do not edit session wiring for vendor special-cases.
4. **Do not recreate** a shared mega `tts_delivery` — delivery stays inside the provider slice.
5. **Do not** add Cargo workspace crates.
6. **FE:** do not re-export domain types from hooks (import from `@/shared/lib/types/*` / feature `lib/`).

---

## Grep gates (required before merge)

```bash
# Provider identity — expect empty outside the Hard-rules allow-list.
# Allowed extra: inline #[cfg(test)] in ai/summary/client.rs and commands/ai.rs
rg "AiProvider::" src-tauri/src --glob "*.rs" `
  -g "!providers/**" -g "!runtime/factories/**" `
  -g "!capabilities/catalog.rs" -g "!config/**" -g "!config_store/**" `
  -g "!commands/config.rs" -g "!meeting/prompts/**" `
  -g "!**/tests.rs" -g "!**/tests/**"

rg "AiProvider::" src-tauri/src/pipeline --glob "*.rs"            # expect: empty
rg "AiProvider::" src-tauri/src/runtime/engine --glob "*.rs"      # expect: empty

# Teardown `.await` must not run under the engine guard: parts are taken
# synchronously (take_teardown_parts / take_direct_*_for_stop) and awaited
# after the guard is dropped. resume_direct_* is sync — allowed under guard.
rg "guard\.(outbound|inbound)\.[a-z_]+\(\)\.await" src-tauri/src --glob "*.rs"   # expect: empty
rg -U "guard\.(resume_direct_(outbound|inbound)_if_enabled|apply_(outbound|inbound)_audio_path|stop_(direct|standby)_[a-z_]*)\([^;]*?\)\s*\.await" src-tauri/src --glob "*.rs"   # expect: empty

# Control plane is bounded via runtime/control_channel.rs (try_send_control).
# Only the OS device-change monitor stays unbounded — allowed:
# audio/device_monitor.rs, runtime/engine/watchdog.rs device_event.
rg "UnboundedSender|UnboundedReceiver|unbounded_channel" src-tauri/src --glob "*.rs"

# A bare `let _ = tx.send(...)` on a bounded Sender compiles as a dropped future.
rg "let _ = [a-z_.]+\.send\((cmd|event|message|t|status)" src-tauri/src --glob "*.rs"   # expect: empty
```

Rust tests: `cargo test --lib` plus `--test meeting_lifecycle --test provider_protocol_ws` ([testing.md](../development/testing.md), [windows.md](../development/windows.md)).

---

## Anti-patterns

- `if config.ai_provider == AiProvider::Soniox` in pipeline/engine/session
- Soniox-named modules outside `providers/soniox/` (prefer provider-TTS names via factory)
- Populating Soniox-only fields on `LiveSetupOptions` for Gemini/OpenAI connects
- Recreating a shared mega `tts_delivery` used by all TTS vendors
- Growing Rust `ai/` or `voice/` re-export facades
- Re-exporting FE domain types from hooks
- A second persist/commit engine beside `SegmentEngine`

---

## Smoke checklist (every backend PR)

- Gemini outbound Translated (STS audio)
- OpenAI outbound Translated
- Soniox outbound Provider TTS + inbound Provider TTS
- ElevenLabs, Fish Audio, or xAI Custom voice outbound **and** inbound hot-switch (mixed vendors allowed)
- Direct passthrough idle CPU
- Meeting summary fallback when live = Soniox
- Overlay hide-from-capture (Win); Mac = best-effort
