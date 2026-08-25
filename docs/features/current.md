# Current — Meetral

**Version:** `4.8.6`  
**Catalog:** [catalog.md](catalog.md)  
**Module boundaries:** [../architecture/overview.md](../architecture/overview.md)  
**Agent router:** [AGENTS.md](../../AGENTS.md)

---

## Product

Real-time meeting interpreter: **You** / **Meeting** columns, Direct or Translate, Gemini / OpenAI / Soniox, optional Custom voice TTS (ElevenLabs, Fish Audio, and/or xAI, both directions, mixed vendors allowed), Notes session, Opus recording, library + FTS, TipTap summaries, artifacts, overlay, tray.

Details and entry files: [catalog.md](catalog.md). Terms: [glossary.md](glossary.md). Unlisted capabilities have **no implementation plan** — do not start them unless the user asks.

## Deferred (do not rewrite unless asked)

1. **`TranslationEngine` sprawl** — multi-file impl; splitting start/stop/watchdog is worthwhile but regresses the live path if done casually.
2. **Inbound/outbound TTS twins** — `InboundProviderTts` still mirrors outbound voice runtime.
3. **Rust `ai/` and `voice/` re-export facades** — thin back-compat; new Rust code imports `capabilities` / `providers`.

## Agent protocol

1. Start at [AGENTS.md](../../AGENTS.md). Read [overview.md](../architecture/overview.md) before changing `src-tauri/src/**` or `src/features/**`.
2. One `SegmentEngine` (`meeting/segment_engine.rs`) for transcript commit. `PhraseCommitter` is not a persist engine.
3. `match AiProvider` only where [overview.md](../architecture/overview.md) Hard rules allow.
4. After a contract change: update this file and [catalog.md](catalog.md). Fold shipped work out of `docs/work/plans/` and delete the plan.
5. Every PR: [testing.md](../development/testing.md) + architecture grep gates.

## Delta

**4.8.6** adds xAI as a third Custom voice TTS vendor. Toolbar stays Engine ↔ Custom; Settings picks ElevenLabs, Fish Audio, or xAI **per column**. Native xAI WebSocket (`wss://api.x.ai/v1/tts`, JSON `text.delta` / `text.done` / `text.clear`, PCM 24 kHz). Built-in + console custom voices — no in-app clone upload. Wire vendor value is `"xai"`. Settings Advanced: latency (`optimize_streaming_latency`) + shared speed only. xAI is utterance TTS: each Soniox `AppendDelta` (translated prefix) is one `text.delta` + `text.done` immediately — do not wait for `Flush` / `turn_complete` / a new transcript segment. One WebSocket per direction (same as ElevenLabs / Fish); later deltas queue until `audio.done`. Do not re-chunk translate text by character count. PCM coalesces ~80 ms then passes through unmodified. First mux emit of each utterance waits for preroll (`PLAYOUT_JITTER_MS` = 200 ms) **and** a second `audio.delta` (or `audio.done` if the clip is one packet) so the DAC does not starve in the first words. The outbound PCM overlap mixer (`elevenlabsPlaybackCrossfade`) is gone — mux upsamples and plays chunks as-is. Stale JSON keys are ignored on load.

**4.8.5** added Fish Audio as a second Custom voice TTS vendor (MessagePack live WS, per-column with ElevenLabs). Wire output value is `"custom"`.

Voice settings persist is a full-document replace. Interpreter voice-output stash must follow the active Engine/Custom patch (same as pipeline modes), and Soniox catalog refresh must not resend the selected TTS model — otherwise later catalog/voice-list writes resurrect Custom voice or the previous model while the drawer is still open.
