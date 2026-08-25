# Live vendors and Chat LLM

Live speech vendors and the summary Chat LLM. Routing rules: [capabilities.md](../architecture/capabilities.md). Hard rules: [overview.md](../architecture/overview.md).

`AiProvider` (`ai/provider.rs`) is **Gemini | OpenAi | Soniox** only. ElevenLabs, Fish Audio, and xAI are voice engines, not `AiProvider`s. Compatible Chat LLM profiles are config data, not a fourth `AiProvider`.

---

## Live vendors

| Vendor | Folder | Live role | Notes STT (`supports_notes_stt_only`) | Engine PCM | Native summary |
|--------|--------|-----------|----------------------------------------|------------|----------------|
| Gemini | `providers/gemini/` | STT + MT + STS | no | `BridgeSts` | yes |
| OpenAI | `providers/openai/` | STT + MT + STS | yes | `BridgeSts` | yes |
| Soniox | `providers/soniox/` | STT + MT; TTS is a **separate** WS | yes | `ProviderTts` | no (summary falls back to Chat LLM) |

Shared live kernel: `providers/shared/live/`. Factory connect: `runtime/factories/live.rs` + `setup.rs`.

**Soniox Engine TTS** (`providers/soniox/tts/`): utterance-per-`AppendDelta` like xAI Custom — each translated prefix is one stream (`config` + `{ text, text_end: true }`); later deltas queue until `terminated`. Fanout still peels prefixes and Flushes on `turn_complete` (shared with Custom EL/Fish on Soniox live); the Engine TTS worker treats `Flush` as a no-op. First mux emit of each generation prerolls `PLAYOUT_JITTER_MS` (200 ms) and a second audio packet (or `audio_end` / `terminated` on a short clip), then coalesces ~80 ms. Do not change fanout to Flush-per-delta (would break Custom EL/Fish). Settings: shared Soniox API key; per-direction TTS model (`sonioxTtsOutboundModel` / `sonioxTtsInboundModel`), voice, and speed. Legacy `sonioxTtsModel` seeds both models on load.

When Meeting → You is Translated or Custom, the pipeline may mix a quiet copy of the meeting floor under TTS inside `spawn_pipeline_audio` — **not** via `DirectRelay`. Use headphones; do not route Local Playback into Meeting Capture.

---

## ElevenLabs (custom voice)

`providers/elevenlabs/` + `runtime/voice_runtime.rs` + `runtime/factories/custom.rs`. Shared API key; per-direction voice / model / stability / similarity / synthesis mode. `PlaybackSource::CustomTts` when that column’s vendor is ElevenLabs.

Do not recreate a shared mega `tts_delivery`. Delivery stays inside the ElevenLabs slice as Gemini/OpenAI text coalescing. Do not point the ElevenLabs worker at Fish’s ElevenLabs-compat URL. Playback mux does not overlap-mix PCM.

---

## Fish Audio (custom voice)

`providers/fishaudio/` + factory spawn in `runtime/factories/custom.rs`. Not a live `AiProvider`. Native API only:

- Live: `wss://api.fish.audio/v1/tts/live`, MessagePack (`start` / `text` / `flush` / `stop`)
- REST: `POST https://api.fish.audio/v1/tts`, `GET /model?self=true`
- Auth: `Authorization: Bearer` + header `model`
- PCM: `format=pcm`, `sample_rate=24000`
- Default model: `s2.1-pro` (`s2.1-pro-free` is a Settings option with no latency SLA)
- Voices: `reference_id` from fish.audio — no in-app `POST /model` training
- Settings: shared API key; per-direction voice / model / latency / temperature; per-direction speed (`fishaudioOutboundSpeed` / `fishaudioInboundSpeed`) and top-p (`fishaudioOutboundTopP` / `fishaudioInboundTopP`). Legacy `fishaudioSpeed` / `fishaudioTopP` seed both directions on load. Preview REST uses outbound knobs.

Per-direction vendor (`outboundCustomVoiceVendor` / `inboundCustomVoiceVendor`). Mixed sessions (You = Fish, Meeting = ElevenLabs) are in scope. One custom voice WebSocket per direction.

---

## xAI (custom voice)

`providers/xai/` + factory spawn in `runtime/factories/custom.rs`. Not a live `AiProvider`. Native API only:

- Live: `wss://api.x.ai/v1/tts` (query: `language`, `voice`, `codec=pcm`, `sample_rate=24000`, `speed`, `optimize_streaming_latency`)
- Client frames: JSON `text.delta` / `text.done` / `text.clear`
- Server frames: JSON `audio.delta` (base64 PCM) / `audio.done` / `audio.clear` / `error`
- REST: `POST https://api.x.ai/v1/tts`, `GET /v1/tts/voices`, `GET /v1/custom-voices`
- Auth: `Authorization: Bearer`
- PCM: `codec=pcm`, `sample_rate=24000` (s16le mono)
- Default voice: `eve` — no model SKU
- Voices: built-in catalog + console custom IDs — no in-app `POST /v1/custom-voices` upload
- Settings: shared API key; per-direction voice / latency; per-direction speed (`xaiOutboundSpeed` / `xaiInboundSpeed`). Legacy `xaiSpeed` seeds both on load. Preview REST uses outbound speed.
- Language: derived from `meeting_language` / `my_language` (mapper + `auto` fallback); not a Settings picker
- Utterance policy (not EL/Fish stream-input): Soniox fanout peels translated prefixes into `AppendDelta` as soon as translate text exists (UI sees the same events). xAI starts TTS on each `AppendDelta` (`text.delta` + `text.done`) — not on `Flush` / `turn_complete` / a new `SegmentEngine` segment. `Flush` only speaks leftover. Do not re-chunk by character count. Gemini / OpenAI unit boundaries stay in their fanout (later). One WebSocket per direction (same as ElevenLabs / Fish). The socket is sequential: later deltas queue until `audio.done`. PCM coalesces ~80 ms then passes through unmodified — no dual-socket reorder, fade, or soft-join. First playout of each utterance prerolls `PLAYOUT_JITTER_MS` (200 ms) and a second `audio.delta` (or `audio.done` on a short clip). Do not delay `text.done` to fake an EL flush.

Per-direction vendor (`outboundCustomVoiceVendor` / `inboundCustomVoiceVendor`). Mixed sessions (You = xAI, Meeting = Fish or ElevenLabs) are in scope. One custom voice WebSocket per direction.

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
