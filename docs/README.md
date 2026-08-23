# Meetral documentation

Living knowledge for developers and coding agents. Start at [AGENTS.md](../AGENTS.md). Human onboarding (clone, audio roles, troubleshooting) stays in [README.md](../README.md).

## Source of truth vs working knowledge

| Tree | Role |
|------|------|
| `architecture/`, `features/`, `integrations/`, `development/` | **Current project knowledge.** Treat as source of truth. |
| `work/plans/`, `work/research/`, `work/bugs/`, `plans/` | **Working knowledge.** May contain assumptions, experiments, or alternatives. Do not implement from here. |

If a living document disagrees with code, **code wins**. Update the document in the same change.

UI tokens and screen rules live in [`design-system/`](../design-system/MASTER.md), not under `docs/`.

## Map

```
docs/
├── architecture/     Module boundaries, pipeline, capabilities, prompts
├── features/         Product now, shipped catalog, glossary
├── integrations/     Live vendors, Direct audio
├── development/      Tests, Windows, macOS, release
├── plans/            Version-cut drafts for review (not SSOT)
└── work/             Plans, research, bugs (not SSOT)
```

| Path | Contents |
|------|----------|
| [architecture/overview.md](architecture/overview.md) | Module map, Hard rules, grep gates |
| [architecture/pipeline.md](architecture/pipeline.md) | Live path, SegmentEngine, timestamps, TipTap |
| [architecture/capabilities.md](architecture/capabilities.md) | ProviderCapabilities, PlaybackSource, Chat LLM |
| [architecture/prompt-conventions.md](architecture/prompt-conventions.md) | Summary prompt hub |
| [features/current.md](features/current.md) | Version, deferred, agent protocol |
| [features/catalog.md](features/catalog.md) | Shipped capabilities → entry files + invariants |
| [features/glossary.md](features/glossary.md) | Product language |
| [integrations/providers.md](integrations/providers.md) | Gemini, OpenAI, Soniox, ElevenLabs, compatible LLM |
| [integrations/direct-audio.md](integrations/direct-audio.md) | Direct passthrough + `keep_direct_audio` |
| [development/testing.md](development/testing.md) | Unit-test contract |
| [development/windows.md](development/windows.md) | Sparse identity / os 15700 |
| [development/macos.md](development/macos.md) | macOS build and BlackHole |
| [development/release.md](development/release.md) | Tag-driven releases |
| [work/README.md](work/README.md) | How to use working docs |
| [plans/](plans/) | Version-cut implementation drafts (fold into living docs, then delete) |

## After a feature ships

1. Update `features/catalog.md` if a capability or its entry files changed.
2. Update `features/current.md` if out-of-scope or deferred changed.
3. Update `features/glossary.md` if a term was coined.
4. Update architecture / integration docs if a boundary or vendor contract changed.
5. Delete the matching file under `work/plans/` or `plans/` once it is folded in.
