# Live vendors and Chat LLM

Live speech vendors and the summary Chat LLM. Routing rules: [capabilities.md](../architecture/capabilities.md). Hard rules: [overview.md](../architecture/overview.md).

`AiProvider` (`ai/provider.rs`) is **Gemini | OpenAi | Soniox** only. ElevenLabs and Fish Audio are voice engines, not `AiProvider`s. Compatible Chat LLM profiles are config data, not a fourth `AiProvider`.

---

## Live vendors

| Vendor | Folder | Live role | Notes STT (`supports_notes_stt_only`) | Engine PCM | Native summary |
|--------|--------|-----------|----------------------------------------|------------|----------------|
| Gemini | `providers/gemini/` | STT + MT + STS | no | `BridgeSts` | yes |
| OpenAI | `providers/openai/` | STT + MT + STS | yes | `BridgeSts` | yes |
| Soniox | `providers/soniox/` | STT + MT; TTS is a **separate** WS | yes | `ProviderTts` | no (summary falls back to Chat LLM) |

Shared live kernel: `providers/shared/live/`. Factory connect: `runtime/factories/live.rs` + `setup.rs`.

When Meeting → You is Translated or Custom, the pipeline may mix a quiet copy of the meeting floor under TTS inside `spawn_pipeline_audio` — **not** via `DirectRelay`. Use headphones; do not route Local Playback into Meeting Capture.

---

## ElevenLabs (custom voice)

`providers/elevenlabs/` + `runtime/voice_runtime.rs` + `runtime/factories/custom.rs`. Shared API key; per-direction voice / model / stability / similarity / synthesis mode. `PlaybackSource::CustomTts` when that column’s vendor is ElevenLabs.

Do not recreate a shared mega `tts_delivery`. Delivery stays inside the ElevenLabs slice as Gemini/OpenAI text coalescing. Do not point the ElevenLabs worker at Fish’s ElevenLabs-compat URL.

---

## Fish Audio (custom voice)

`providers/fishaudio/` + factory spawn in `runtime/factories/custom.rs`. Not a live `AiProvider`. Native API only:

- Live: `wss://api.fish.audio/v1/tts/live`, MessagePack (`start` / `text` / `flush` / `stop`)
- REST: `POST https://api.fish.audio/v1/tts`, `GET /model?self=true`
- Auth: `Authorization: Bearer` + header `model`
- PCM: `format=pcm`, `sample_rate=24000`
- Default model: `s2.1-pro` (`s2.1-pro-free` is a Settings option with no latency SLA)
- Voices: `reference_id` from fish.audio — no in-app `POST /model` training

Per-direction vendor (`outboundCustomVoiceVendor` / `inboundCustomVoiceVendor`). Mixed sessions (You = Fish, Meeting = ElevenLabs) are in scope. One custom voice WebSocket per direction.

---

## Compatible Chat LLM (summary only)

`providers/compatible/` + `config/custom_llm.rs`. One slice covers Ollama / LM Studio / llama.cpp / vLLM. Profiles are **data**: `summary_custom_profile_id` → `LlmSelection::{BuiltIn, Custom}`.

- Chat-only — no embeddings.
- Save-time validation: 1 JSON-mode generate + 1 streaming generate must pass.
- Keychain account `custom_llm:<id>`; orphan GC in `config_store/stored.rs`.

---

## Adding a live vendor

Allowed: `providers/<name>/` (new folder), `runtime/factories/*` (one registration), `capabilities/catalog.rs` (caps row), `config/**` + `config_store/**` (settings/keys), `commands/config.rs` if IPC defaults need a field.

Forbidden: `pipeline/**`, `runtime/engine/**`, `runtime/playback_mux.rs` vendor `match`. Do not special-case the new name in session wiring.

Tests: catalog normalize/migrate; factory setup; grep gates in [overview.md](../architecture/overview.md). Update [catalog.md](../features/catalog.md) if the user-visible vendor list changed.
