# Testing

Unit tests follow **colocated** placement and **FIRST** quality rules.
Architecture boundaries: [overview.md](../architecture/overview.md).

## Placement

| Layer | Default | Shared helpers only |
|-------|---------|---------------------|
| **FE** | `Foo.ts` + `Foo.test.ts` next to each other under `src/features/` or `src/shared/` | `src/test/setup.ts`, `src/test/fixtures/` |
| **BE** | Inline `#[cfg(test)] mod tests` next to the code; sibling `tests/` under the same module when the suite is large | `src-tauri/src/test_support/` (`#[cfg(test)]`) |

**Do not** use a repo-wide `__tests__/` mirror tree. Crate-level integration tests live in `src-tauri/tests/` (`meeting_lifecycle.rs`, `provider_protocol_ws.rs`). They do not replace colocated unit tests.

### Naming

- FE: `<module>.test.ts`, hooks `<hook>.test.ts`, integrity suites `*.integrity.test.ts`
- BE: `mod tests` or `*_tests.rs`; case names `when_x_then_y` / behavior-focused

## Quality checklist (FIRST + AAA)

| Rule | Practice |
|------|----------|
| **Fast** | No real WebSocket, WASAPI, or network. Mock `invoke`. Prefer pure functions and table-driven matrices. |
| **Independent** | Fresh fixtures per test; no shared mutable globals across cases. |
| **Repeatable** | Deterministic; mock time/random when needed. |
| **Self-checking** | `expect` / `assert_eq!` — no eyeball-only logs. |
| **Focused** | One behavior or contract per test; table-driven for routing matrices. |
| **AAA** | Arrange → Act → Assert (comments optional). |
| **Behavior, not impl** | Assert public outputs / contracts, not private fields unless they *are* the contract. |
| **Right level** | Unit here; E2E/Playwright out of scope for this suite. |

## Backend

### Fixtures

```rust
use crate::test_support::{sample_config, sample_devices, sample_event};
```

Use `test_support` when the same fixture is needed in ≥2 modules. Do not re-copy `sample_config` / `sample_devices`.

### Architecture test imports

- Route / capability assertions: `crate::capabilities::*`
- Factory wiring: `crate::runtime::factories::*`
- **Do not** assert routing via `crate::voice::spawn_*` — factories are the match site.

### Commands

```bash
cd src-tauri
cargo test --lib
cargo test --lib capabilities
cargo test --lib runtime::factories
cargo test --test meeting_lifecycle --test provider_protocol_ws
```

Optional coverage (local): `cargo llvm-cov --lib` when the toolchain is installed — not required in CI.

Full `cargo test` (including `--bin meetral`) can fail on Windows without package identity — [windows.md](windows.md).

## Frontend

`vitest.config.ts` loads `src/test/setup.ts`, which mocks `@tauri-apps/api/core` `invoke` and runs RTL `cleanup` after each test.

```ts
import { invoke } from "@tauri-apps/api/core";
import { vi } from "vitest";

vi.mocked(invoke).mockResolvedValue(...);
```

Shared config/status fixtures: `@/test/fixtures/config` (`baseConfig`, `baseStatus`).

```bash
npm test                 # vitest run
npm run test:coverage    # vitest + v8 coverage (pipeline + config gates)
npm run test:all         # FE + cargo test
```

Coverage `include` in `vitest.config.ts` is scoped to pipeline/config **lib, hooks, and api** (not UI components) with low thresholds (lines/statements ~40%). Raise gradually; do not chase 100%.

## CI

Keep CI on the existing combined suite:

```bash
npm run test:all
```

That runs `vitest run` then `cargo test` in `src-tauri`. Optional local extras:

- `npm run test:coverage` — FE coverage report (not required to block merge until thresholds are stable)
- `cargo llvm-cov --lib` — BE coverage when installed

Do not add Playwright/E2E to this unit gate.

## Current coverage focus

**BE:** capabilities routing + catalog normalize/migrate; factories setup/summary; playback mux (route + drop/drain); voice_runtime pure helpers; engine recovery predicates / audio backoff; provider protocol/delivery suites.

**FE:** pipeline status/labels/segment state/reducer streaming paths; hydrate + `useMeetingChanged` fan-out; meeting/config/voice/pipeline API wrappers; ai catalog; soniox context; voice TTS status; production DevTools shortcut matrix.

Still deferred (integration-heavy): full voice switch with AppHandle, live bridge connect/WASAPI, SettingsDrawer UI trees.

## What not to do

- Mass-move tests into `__tests__/`
- Real provider WebSocket or audio device I/O in unit tests
- Large UI snapshots / mock entire component trees then assert placeholders
- Vanity 100% line coverage — use coverage as a signal; raise thresholds gradually
- New `#[path = "..."]` shims for test modules
