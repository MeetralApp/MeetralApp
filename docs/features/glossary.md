# Glossary

Product language for Meetral. Prefer these terms in code comments, UI copy, and docs.

| Term | Meaning |
|------|---------|
| **You** | Outbound column — the user's speech, captured from **Your microphone**, played into **Teams mic feed**. |
| **Meeting** | Inbound column — remote participants, captured from **Meeting capture**, played to **Local playback**. |
| **Direct** | Passthrough with no live API. Mic → Teams feed; meeting → headphones. |
| **Translate** | Live STT + MT via Gemini, OpenAI, or Soniox, then TTS / custom voice. |
| **Notes session** | `SessionMode::Notes` — single-language capture UX. Not a second commit engine. OpenAI + Soniox only. |
| **Interpreter session** | Default `SessionMode::Interpreter` — bilingual You / Meeting. |
| **Segment** | Stabilized transcript unit persisted by `SegmentEngine`. |
| **Turn** | Provider-protocol event (phrase / `turnComplete` / accumulator). Synthesized; not the persist unit. |
| **Wall time** | Unix ms on `meeting_record.started_at_ms` / `ended_at_ms`. |
| **Meeting-relative time** | ms from meeting start on `transcript_segment.started_at_ms` / `ended_at_ms`. |
| **`ProviderCapabilities`** | Catalog flags for a live vendor (`capabilities/catalog.rs`). Routing input. |
| **`PlaybackSource`** | `BridgeSts` \| `ProviderTts` \| `CustomTts` — which PCM feeds Translated playback. |
| **Commit vs persist** | Protocol layers commit *turns*; only `SegmentEngine` persists *segments*. |
| **TipTap SSOT** | Summary truth is `generated_json`, not a parallel markdown/chat document. |
| **Bridge STS** | Speech-to-speech PCM from the Gemini/OpenAI live bridge. |
| **Provider TTS** | Separate TTS WebSocket (Soniox). |
| **Custom voice** | Custom voice TTS path (`PlaybackSource::CustomTts`) via ElevenLabs, Fish Audio, and/or xAI, both directions. Toolbar is Engine vs Custom; vendor is a Settings control per column. |
| **`keep_direct_audio`** | When true, Direct relay stays up on idle / non-translating directions. |

**Do not use as types:** CommitEngine (the type is `SegmentEngine`), LiveCaps (the type is `ProviderCapabilities`).
