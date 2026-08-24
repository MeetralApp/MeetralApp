# Current — Meetral

**Version:** `4.8.5`  
**Catalog:** [catalog.md](catalog.md)  
**Module boundaries:** [../architecture/overview.md](../architecture/overview.md)  
**Agent router:** [AGENTS.md](../../AGENTS.md)

---

## Product

Real-time meeting interpreter: **You** / **Meeting** columns, Direct or Translate, Gemini / OpenAI / Soniox, optional Custom voice TTS (ElevenLabs and/or Fish Audio, both directions, mixed vendors allowed), Notes session, Opus recording, library + FTS, TipTap summaries, artifacts, overlay, tray.

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

**4.8.5** adds Fish Audio as a second Custom voice TTS vendor. Toolbar stays Engine ↔ Custom; Settings picks ElevenLabs or Fish Audio **per column**. Native Fish WebSocket (`wss://api.fish.audio/v1/tts/live`, MessagePack PCM 24 kHz). Custom voices on fish.audio — no in-app `POST /model` upload. Wire value is `"custom"` (hard rename — no legacy clone aliases).

Voice settings persist is a full-document replace. Interpreter voice-output stash must follow the active Engine/Custom patch (same as pipeline modes), and Soniox catalog refresh must not resend the selected TTS model — otherwise later catalog/voice-list writes resurrect Custom voice or the previous model while the drawer is still open.
