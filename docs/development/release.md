# Release

Meetral releases are tag-driven. Pushing a `v*` tag runs
[`.github/workflows/release.yml`](../../.github/workflows/release.yml), which builds
Windows (x64) and macOS (arm64) bundles and attaches them to a **draft** GitHub
Release. Artifacts are currently **unsigned** — see "Signing" below.

## Cut a release

1. Bump the version in all three places (keep them in sync):
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `[package].version`
   - `src-tauri/tauri.conf.json` → `version`
2. Update the **Delta** section in [features/current.md](../features/current.md).
3. Commit on `main`, then tag and push:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

4. Wait for the `Release` workflow, then review the auto-created **draft**
   release on GitHub, edit notes if needed, and publish manually.

CI also builds `tauri build` on pushes to `main` and on `v*` tags (see
`ci.yml`), so a tagged commit has already produced green bundles once.

`npm run check:version` fails if the three version fields drift.

## Signing (deferred)

The scaffold ships unsigned bundles. Enabling signing is a follow-up task with
an assigned owner; when picked up:

### Tauri updater signing

1. Generate a keypair locally:

```bash
npm run tauri signer generate -- -w ~/.tauri/meetral.key
```

2. Add GitHub secrets:
   - `TAURI_SIGNING_PRIVATE_KEY` — contents of `~/.tauri/meetral.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the password chosen above
3. Wire them as `env` on the `tauri-action` step in `release.yml`.
4. Add the public key to `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
   when the updater plugin is introduced.

### Platform codesigning / notarization (macOS)

- `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` (Developer ID Application, p12)
- `APPLE_SIGNING_IDENTITY`
- `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH` (App Store Connect API, for notarization)

Windows Authenticode signing is also deferred; pick a certificate provider when
this task is scheduled.

Until then, users must bypass Gatekeeper/SmartScreen warnings for unsigned
artifacts, and no updater feed is published.
