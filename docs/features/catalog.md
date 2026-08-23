# Product catalog

Living feature map for Meetral `4.8.5`. Written from **code**. Not a backlog. Unlisted capabilities have no implementation plan — [current.md](current.md).

How to read: pick the row you are changing, open the entry files, obey the invariant. UI tokens: `design-system/pages/` under **Read**.

| Capability | User-visible | Entry | Read | Invariant |
|------------|--------------|-------|------|-----------|
| Live Interpreter | You / Meeting columns; Direct vs Translate | `src/features/pipeline`, `pipeline/{inbound,outbound}`, `runtime/engine` | `design-system/pages/live.md`, [pipeline.md](../architecture/pipeline.md) | Branch on `ProviderCapabilities` / `PlaybackSource`, not `AiProvider` |
| Direct passthrough | Mic → Teams feed; meeting → headphones; no live API | `runtime/direct_relay.rs`, `audio/backend/{windows,macos}/direct_passthrough.rs` | [direct-audio.md](../integrations/direct-audio.md) | Do not restore WASAPI busy-loop silence fill |
| Transcript commit | Stabilized segments in UI + SQLite | `meeting/segment_engine.rs` | [pipeline.md](../architecture/pipeline.md) flush callers | Only persist owner. Two callers (`flush_live_transcript_segments` = stop-direction; `finalize_meeting_transcripts` = end-meeting), same `flush_direction`. Protocol layers do not persist |
| Live vendors | Gemini, OpenAI, Soniox | `providers/{gemini,openai,soniox}`, `runtime/factories` | [providers.md](../integrations/providers.md), [capabilities.md](../architecture/capabilities.md) | New vendor = new folder + factory registration |
| Custom voice | ElevenLabs and/or Fish Audio, **both** directions; per-direction vendor | `providers/{elevenlabs,fishaudio}`, `runtime/voice_runtime.rs`, `runtime/factories/custom.rs` | [capabilities.md](../architecture/capabilities.md) `PlaybackSource` table | Toolbar Engine ↔ Custom only. One API key per vendor. Mixed You/Meeting vendors in scope. No in-app Fish training |
| Overlay | Transcript window; hide-from-capture (Win) | `overlay/`, `src/features/overlay` | `design-system/pages/overlay.md` | Transcript only |
| Tray / hotkeys | Close-to-tray; O/T | `src-tauri/src/tray.rs`, `lib.rs` | [README.md](../../README.md) Usage | — |
| Notes session | Single-language capture UX | `SessionMode`, `NotesPipelineToolbar`, overlay Notes path | `design-system/pages/live.md` | Not a second commit engine. Notes STT: OpenAI + Soniox yes; Gemini no (`supports_notes_stt_only`) |
| Recording | Dual-direction Ogg Opus + detail player | `meeting/recording`, `MeetingAudioPlayer` | `design-system/pages/meeting-detail.md` | Dual persist; mix at playback (`byte_size` on chunks) |
| Library | Folders, current/recent, FTS | `src/features/meeting/library`, `meeting/store`, `meeting/store/fts.rs` | `design-system/pages/history.md` | Schema: `meeting/migrations/001_initial.sql` |
| Summary | TipTap `generated_json` SSOT | `summary_service`, `src/features/meeting/detail/lib/summaryDoc/**` | [pipeline.md](../architecture/pipeline.md), `design-system/pages/meeting-detail.md` | Citations = TipTap `segmentId` attrs, not a chat product |
| Artifacts | Decisions / actions from summary | `meeting/store/artifacts`, `ArtifactsPanel` | `design-system/pages/meeting-detail.md` | Citation chips stay under meeting detail |
| Summary templates | `meeting_brief`, `executive_summary`, `action_items_only`, `decisions_log`, `standup` | `meeting/prompts/templates` | [prompt-conventions.md](../architecture/prompt-conventions.md) | Prompts live in this hub only |
| Custom chat LLM | OpenAI-compatible profiles for **summary** | `providers/compatible`, `config/custom_llm.rs` | [capabilities.md](../architecture/capabilities.md) Chat LLM | Data, not a new vendor folder per Ollama. Chat-only — no embeddings |
| Soniox context | Boost terms / profile | `providers/soniox/context.rs`, `src/features/voice/components/soniox-context/SonioxContextSettings.tsx` | `design-system/pages/settings.md` | — |
| Config / secrets | Settings + keychain | `config/`, `config_store/`, `secret.rs`, `src/features/config` | `design-system/pages/settings.md` | Unknown JSON keys are ignored on load |
| Windows identity | Taskbar grouping / notifications | `scripts/sparse-identity`, `windows_identity.rs` | [windows.md](../development/windows.md) | Do not `Add-AppxPackage` without `-ExternalLocation` |
