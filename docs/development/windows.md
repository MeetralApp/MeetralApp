# Windows development

Sparse MSIX identity (taskbar grouping / notifications) vs unpackaged `cargo run`. Read this before treating os **15700** as an app bug.

**App id (install / release):** `com.meetral.desktop`. Sparse package **Name** in the Appx manifest is `Meetral`.
**App id (local dev):** `com.meetral.desktop.dev` via `npm run tauri:dev` → `%APPDATA%\com.meetral.desktop.dev\`.

## Durable app data (Roaming)

Config, the meeting library (`meetings.db`), default recordings, and logs are stored under real roaming AppData — not under `Packages\Meetral_*\LocalCache`:

- Release / install: `%APPDATA%\com.meetral.desktop\`
- `npm run tauri:dev`: `%APPDATA%\com.meetral.desktop.dev\`

Owner: `src-tauri/src/app_data.rs`. On Windows it resolves Roaming with `KF_FLAG_NO_PACKAGE_REDIRECTION` and prefixes paths with `\\?\` so package AppData redirection cannot send writes back into LocalCache. The sparse manifest also declares `unvirtualizedResources` + `FileSystemWriteVirtualization=disabled`; **re-register** sparse identity after rebuilding the `.msix` so that capability takes effect. There is no automatic copy from LocalCache — a first launch after this change starts with empty settings/library unless you copy files yourself.

## `The process has no package identity` (os error 15700)

Windows refused `CreateProcess`. Cargo’s “never executed” means the process did not start — this is not `windows_identity.rs` (that runs after launch).

Typical cause: a leftover **Meetral** Appx package (often `Meetral_*.0_neutral__*` under `WindowsApps`) from a prior sparse install **without** `-ExternalLocation`. Then `target\debug\meetral.exe` and even `target\debug\deps\meetral-*.exe` fail to launch.

**Unblock unpackaged `tauri:dev` / `cargo run` / `cargo test --bin meetral`:**

```powershell
Get-AppxPackage -Name Meetral | Remove-AppxPackage
```

Then retry `npm run tauri:dev`.

**Optional — run the debug exe with package identity** (taskbar grouping). After `meetral.exe` exists in `src-tauri\target\debug`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\sparse-identity\Register-SparseIdentity.ps1
```

That script **removes** any existing Meetral package and registers sparse identity with `ExternalLocation = src-tauri\target\debug`. First run may UAC-prompt to trust the MeetralDev cert.

Do **not** `Add-AppxPackage` the `.msix` without `-ExternalLocation`. That installs into WindowsApps and brings 15700 back.

## Tests on this machine

Prefer `cargo test --lib` plus `--test meeting_lifecycle --test provider_protocol_ws` (see [AGENTS.md](../../AGENTS.md)). Full `cargo test` includes `--bin meetral` and hits the same CreateProcess identity check.

`npm run tauri:dev` does **not** run `npm run build:sparse`. Identity is a separate Windows registration, not a Rust compile flag. Dev uses identifier `com.meetral.desktop.dev` (separate AppData from install).
