# Capabilities and voice routing

Pipeline, session, mux, and engine must not `match AiProvider`. They read **`ProviderCapabilities`** (catalog) and **`PlaybackSource`** (TTS path). Helpers live in `capabilities/tts.rs` (`live_caps()`, `outbound_playback_source()`, `inbound_playback_source()`).

The catalog type is `ProviderCapabilities` in `capabilities/catalog.rs` (serialized camelCase for the frontend). There is no `LiveCaps` type in code.

---

## ProviderCapabilities

```rust
supports_vad_config
supports_echo_target_language
live_upload_sample_rate
supports_native_summary
uses_separate_tts          // Soniox = true
bridge_emits_playback_audio // !uses_separate_tts
supports_notes_stt_only    // OpenAI + Soniox = true; Gemini = false
```

| Live provider | Engine voice PCM | Custom voice (ElevenLabs, Fish Audio, or xAI) |
|---------------|------------------|-----------------------------------------------|
| Gemini / OpenAI | `BridgeSts` (STS from the live bridge) | `CustomTts` — **both** outbound (You→Meeting) and inbound (Meeting→You) |
| Soniox | `ProviderTts` (separate TTS WebSocket) | `CustomTts` — **both** directions |

```rust
enum PlaybackSource { BridgeSts, ProviderTts, CustomTts }
```

Shared custom voice path (`PlaybackSource::CustomTts`) on **both** You and Meeting columns. Per-direction `CustomVoiceVendor` (ElevenLabs, Fish Audio, or xAI). One API key per vendor; per-direction voice/knobs. Pipeline / mux / engine still branch on `PlaybackSource` / `VOICE_ENGINE_CUSTOM`, never vendor identity.

Notes session (`SessionMode::Notes`) is STT-only. Gemini cannot run Notes (`supports_notes_stt_only = false`). Validation: `config/app_config_validate.rs`.

Vendor folders and how to add one: [providers.md](../integrations/providers.md).

---

## Chat LLM boundaries

Chat LLMs are abstracted behind `ChatLlmProvider` (`ai/llm/client.rs`). `SummaryLlmClient` is the retry/observability wrapper and holds `Box<dyn ChatLlmProvider>`. Retry decisions use `LlmErrorKind` (classified at the provider slice), not string matching.

| Concern | Zone |
|---------|------|
| Trait + `ChatRequest` + `LlmError`/`LlmErrorKind` | `ai/llm/` |
| Vendor impls (`GeminiChatLlm`, `OpenAiChatLlm`, `CompatibleChatLlm`) | `providers/<name>/` |
| Dispatch (`chat_llm_for`, `*_for_selection`) | `runtime/factories/` |
| `LlmCapabilities` descriptors | `capabilities/catalog.rs` |
| Custom profiles + `LlmSelection` resolver (`summary_llm_selection()`) | `config/` (`custom_llm.rs`, `app_config.rs`) |
| Keychain `custom_llm:<id>` + encrypted-at-rest + orphan GC | `secret.rs`, `config_store/stored.rs` |

**Custom OpenAI-compatible profiles are data, not code** — one `providers/compatible/` slice covers Ollama / LM Studio / llama.cpp / vLLM. Selection = `summary_custom_profile_id` (config) → `LlmSelection::{BuiltIn, Custom}`. Domain services receive a ready `SummaryLlmClient` and never see provider identity.

Chat-only — no embeddings.

**Capability degradation** (caller degrades; no provider special-casing):

| Capability | false/absent behavior |
|------------|----------------------|
| `streaming` | non-stream `generate` path |
| `json_mode` | prompt-only JSON + tolerant parse |
| `auth = None/Optional` | gate satisfied without a key (local servers) |
| `model_source = Freeform` | no allowlist clamp — typo caught by save-time test call |

Save-time validation is mandatory: 1 JSON-mode generate + 1 streaming generate must pass before a profile persists.
