# Contributing

Thanks for looking. Meetral is a live meeting interpreter (Tauri 2, React, Rust).

## Before you change code

1. Read [AGENTS.md](AGENTS.md) if you are using a coding agent. Humans can start at [docs/architecture/overview.md](docs/architecture/overview.md) and the one row in [docs/features/catalog.md](docs/features/catalog.md) for the capability you are touching.
2. Do not implement from [docs/work/](docs/work/) or [docs/plans/](docs/plans/). Those notes are not the product contract.
3. Unlisted capabilities have no implementation plan. Open an issue before starting one.

## Rules that keep the app one system

- Route live pipeline, session, mux, and engine on `ProviderCapabilities` and `PlaybackSource`, not on which vendor it is.
- Persist transcript segments only through `SegmentEngine`. Protocol helpers do not write SQLite.
- A new live vendor is a new folder under `src-tauri/src/providers/` plus one factory registration.
- Prompts live only in `src-tauri/src/meeting/prompts/`.
- UI follows `design-system/`. Page files override `MASTER.md`.

## Checks

```bash
npm install
npm run check
```

That is the local CI gate (version sync, lint, knip, frontend build and tests, `cargo test`, rustfmt, clippy). While iterating, `npm test` and `cargo test --lib` from `src-tauri` are enough. Colocate tests next to the code.

Windows: full `cargo test` can fail with os error 15700 if a Meetral Appx package was installed without `-ExternalLocation`. See [docs/development/windows.md](docs/development/windows.md).

## Pull requests

- Describe the user-visible behavior and why it changed.
- If a catalog entry or public contract changed, update [docs/features/catalog.md](docs/features/catalog.md) and [docs/features/current.md](docs/features/current.md) in the same PR.
- Do not commit `.env`, certificates, or API keys.

By submitting a contribution you agree it is licensed under the [MIT License](LICENSE).

## Copyright

The license copyright holder is **Meetral** (`LICENSE`, installer copyright). Git author metadata may still say `iamnhaan`; that is attribution, not a second license.
