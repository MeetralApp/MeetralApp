# Live pipeline

Canonical live path. Both directions share one `TranslationEngine` (`runtime/engine/`). Boundaries: [overview.md](overview.md). Capabilities: [capabilities.md](capabilities.md).

```
Capture (WASAPI / CoreAudio, 48 kHz)
  → resample to provider upload rate
  → LiveBridgeHandle (Gemini | OpenAI | Soniox)
       STT + MT inside the live API
  → transcript fanout (UI + optional TTS)
  → SegmentEngine.process()     ← ONE commit pipeline
  → TranscriptDbWriter (async SQLite)
  → TTS (bridge STS | provider TTS | custom voice TTS)
  → playback_mux
  → local playback / Teams mic feed
```

Direct mode bypasses the live API via `runtime/direct_relay.rs` — [direct-audio.md](../integrations/direct-audio.md).

`PipelineState` (`runtime/engine/types.rs`): `Off` | `Direct` | `Starting` | `Stopping` | `Active` | `Error`.

---

## SegmentEngine

`meeting/segment_engine.rs` is the **only** transcript-stabilization owner (sentence / turn / gap / flush).

Provider protocol layers synthesize turn events; they do **not** persist segments:

- OpenAI `PhraseCommitter` — `providers/openai/protocol.rs`
- Gemini `turnComplete`
- Soniox token accumulator

TTS sentence splitting in `voice/shared/tts_text/sentence.rs` is a separate concern. It uses the same terminator set (including CJK `。！？`).

Do not add a second persist engine.

Two **callers**, one flush:

| Caller | When | Why |
|--------|------|-----|
| `TranslationEngine::flush_live_transcript_segments` | Stop a live **direction** (Translate off, device lost, bridge fatal, quit stop) | Persist trailing live text **before** aborting the provider bridge |
| `meeting::finalize_meeting_transcripts` | **End meeting** (`end_meeting` IPC, `end_active_meeting` on quit) | Persist while `ActiveMeetingId` is still set, then mark the meeting Ended |

Both call `SegmentEngine::flush_direction` + `persist_segment_commits_sync`. A second flush on an empty/sealed buffer is a no-op. Keep both entry points — do not collapse stop-direction into end-meeting (Translate can stop mid-meeting) and do not skip finalize (Live End still ends the meeting if audio stop fails).

---

## Meeting timestamps

| Field | Clock | Meaning |
|-------|--------|---------|
| `meeting_record.started_at_ms` / `ended_at_ms` | **Wall** (Unix ms) | Library calendar + Live header timer (`useMeetingElapsed`) |
| `transcript_segment.started_at_ms` / `ended_at_ms` | **Meeting-relative** ms | `0` = meeting start; same axis as header elapsed; seek/audio use this |
| Live write path | Mono anchor | `meeting::time::mono_anchor_for_meeting` maps wall elapsed → process mono so relative stays continuous across restart |

See `src-tauri/src/meeting/time.rs`.

---

## Meeting summary (TipTap SSOT)

| Field | Role |
|-------|------|
| `generated_json` | TipTap document JSON — **SSOT** for review + edit |
| Display | TipTap UI; citation jump via TipTap `segmentId` attrs |

LLM brief schema is parse-only on the generate path; seed TipTap then persist. Persist: `MeetingStore::save_summary` / `update_summary_doc`.

**SummaryKit** (FE document type — not open StarterKit): vocabulary = generate seed (`heading` L3, `bulletList`/`listItem`, `paragraph`, `bold`, `citation`). Styles live on extension `HTMLAttributes` (`summaryKit.ts`). Edit: `/` structure suggestion (Section / Bullet), `@` + transcript click for citations, BubbleMenu Bold on selection. View + edit share the same kit (`SummaryDocEditor` / `SummaryDocView`). FE: `src/features/meeting/detail/lib/summaryDoc/**`.

Transcript-segment citations on summaries and artifacts use `AnchorTimeBadge`. They are not a chat product.

Prompt hub: [prompt-conventions.md](prompt-conventions.md).

---

## Frontend layout

```
src/
  app/
  features/{pipeline,config,ai,voice,audio,meeting/{library,detail},overlay}
  shared/{ui,components,layout,hooks,context,lib}
```

Live runtime context: `PipelineRuntimeProvider`. Voice IPC lives under `src/features/voice`. Pipeline IPC: `src/features/pipeline/api/pipelineApi.ts`.
