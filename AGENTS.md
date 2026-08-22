# Agents

Meetral is a **live meeting interpreter** (Tauri 2 + React + Rust): bidirectional **You** / **Meeting** translation, meeting library, and summaries. Version **4.8.4**.

Shipped surface: [docs/features/catalog.md](docs/features/catalog.md). Do not start capabilities that are not in the catalog unless the user asks — there is no implementation plan for unlisted work.

## Before you implement

1. Read this file.
2. Read [docs/features/current.md](docs/features/current.md) (product now, deferred).
3. Open the **one row** in [docs/features/catalog.md](docs/features/catalog.md) for the capability you are changing.
4. If you touch `src-tauri/src/**` or `src/features/**`, read [docs/architecture/overview.md](docs/architecture/overview.md).
5. Then read only the extra doc that matches the task (table below).
6. Prefer **code** over any document if they disagree.

Do **not** implement from [docs/work/](docs/work/). Plans, research, and bug notes are working knowledge, not architectural truth.

## When to read `docs/`

| Task | Read |
|------|------|
| Module boundaries, `match AiProvider`, grep gates | [architecture/overview.md](docs/architecture/overview.md) |
| Live path, `SegmentEngine`, timestamps, TipTap summary | [architecture/pipeline.md](docs/architecture/pipeline.md) |
| `ProviderCapabilities`, `PlaybackSource`, Chat LLM | [architecture/capabilities.md](docs/architecture/capabilities.md) |
| Summary prompts | [architecture/prompt-conventions.md](docs/architecture/prompt-conventions.md) |
| UI | [design-system/MASTER.md](design-system/MASTER.md) + `design-system/pages/<screen>.md` |
| Direct / `keep_direct_audio` | [integrations/direct-audio.md](docs/integrations/direct-audio.md) |
| New live vendor or custom summary LLM | [integrations/providers.md](docs/integrations/providers.md) |
| Tests | [development/testing.md](docs/development/testing.md) |
| Windows launch / os 15700 | [development/windows.md](docs/development/windows.md) |
| macOS build / BlackHole | [development/macos.md](docs/development/macos.md) |
| Cut a release | [development/release.md](docs/development/release.md) |
| Domain terms | [features/glossary.md](docs/features/glossary.md) |
| Docs map | [docs/README.md](docs/README.md) |

## Repository

```
src/                 React app — features/{pipeline,config,ai,voice,audio,meeting,overlay}
src/shared/          UI primitives, layout, hooks, types
src-tauri/src/       Rust crate (one crate — no workspace split)
  capabilities/      ProviderCapabilities + PlaybackSource + routing helpers
  providers/         gemini, openai, soniox, elevenlabs, compatible, shared/live
  runtime/           engine, factories, voice_runtime, playback_mux, direct_relay
  pipeline/          inbound / outbound session wiring
  meeting/           SQLite library, SegmentEngine, summary, prompts, recording
  audio/             WASAPI / CoreAudio + Direct passthrough
  overlay/, tray.rs, commands/, config/, config_store/
design-system/       UI SSOT (tokens + page overrides)
docs/                Project knowledge — see docs/README.md
```

`src-tauri/src/ai/` and `src-tauri/src/voice/` are **thin Rust re-export facades**. Do not grow them. New Rust code imports `capabilities/` and `providers/`.

`src/features/ai/` and `src/features/voice/` are **real frontend features** (catalog, keys, TTS UI). Those are not facades.

## Architecture principles

- Route live/session/mux/engine on **`ProviderCapabilities`** / **`PlaybackSource`** / factory helpers — never `AiProvider` identity.
- One transcript-stabilization owner: `meeting/segment_engine.rs` (`SegmentEngine`).
- New live vendor = new folder under `providers/` + one factory registration. Do not special-case vendors in pipeline/session/engine.
- `match AiProvider` / `== AiProvider::Soniox` only on the Hard-rules allow-list in [architecture/overview.md](docs/architecture/overview.md).
- Prompts live only in `meeting/prompts/`. Provider HTTP must not embed prompt text.

## MUST

- Branch pipeline, session, mux, and engine on capabilities — not provider enums.
- Persist transcript segments only through `SegmentEngine`. Two callers share `flush_direction`: `flush_live_transcript_segments` (stop-direction) and `finalize_meeting_transcripts` (end-meeting) — keep both; see [pipeline.md](docs/architecture/pipeline.md). `PhraseCommitter`, Gemini `turnComplete`, and the Soniox accumulator do not persist.
- After a user-visible or entry-file change: update `docs/features/current.md` and/or `docs/features/catalog.md`.
- Run the test commands and grep gates before claiming done.
- Colocate unit tests (`Foo.ts` + `Foo.test.ts`; Rust `#[cfg(test)]` next to the code).

## MUST NOT

- Add a second persist/commit engine.
- Grow the Rust `ai/` or `voice/` facades.
- Recreate a shared mega `tts_delivery` used by all TTS vendors.
- Add Cargo workspace crates.
- Restore WASAPI busy-loop silence fill on Direct passthrough.
- `Add-AppxPackage` a Meetral `.msix` without `-ExternalLocation` (Windows os 15700).
- Treat `docs/work/**` as product or architecture SSOT.
- Rewrite deferred internals unless the user asks: `TranslationEngine` file sprawl, inbound/outbound TTS twins, Rust `ai/`/`voice/` facades.

## Conventions

- **Rust:** vendor code under `providers/<name>/`; factories in `runtime/factories/`; IPC in `commands/` stays thin.
- **Frontend:** feature UI under `src/features/<feature>/`; shared types from `@/shared/lib/types/*` — do not re-export domain types from hooks.
- **UI:** follow `design-system/` (OLED dark default, semantic tokens, Lucide only). Page files override MASTER.
- **Config:** unknown JSON keys are ignored on load (`StoredConfig` does not deny unknown fields).

## Testing

```bash
npm test
cd src-tauri && cargo test --lib
cd src-tauri && cargo test --test meeting_lifecycle --test provider_protocol_ws
```

Grep gates: [docs/architecture/overview.md](docs/architecture/overview.md). Full `cargo test` (including `--bin meetral`) can fail on Windows without package identity — [development/windows.md](docs/development/windows.md).

No real WebSocket, WASAPI, or network in unit tests. Details: [development/testing.md](docs/development/testing.md).

## Definition of Done

- Behavior matches the catalog row invariant and the Hard rules.
- `npm test` and `cargo test --lib` (+ named integration tests when you touched meeting/provider protocol) pass.
- Grep gates are clean for the allow-list.
- UI changes follow the matching `design-system/pages/` file.
- Living docs updated if the contract or entry files changed.
- No leftover plan in `docs/work/plans/` for work that already shipped — fold into living docs, then delete the plan.
