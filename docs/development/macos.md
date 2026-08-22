# macOS build

Production builds target **Apple Silicon (arm64)** on **macOS 13+**.

App identity: **Meetral** (`com.meetral.app`) · Dock/menu name from `Info.plist` + `bundle.macOS.bundleName`.

## Requirements

- macOS 13+ on Apple Silicon (or cross-compile from Intel Mac with Rust `aarch64-apple-darwin` target)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/)
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/)
- [BlackHole 2ch + BlackHole 16ch](https://existential.audio/blackhole/) for virtual audio routing with Teams

## Config (already in repo)

| File | Role |
|------|------|
| `src-tauri/tauri.conf.json` → `bundle.macOS` | `minimumSystemVersion` 13.0, `hardenedRuntime`, `bundleName` Meetral, DMG layout, entitlements + Info.plist paths |
| `src-tauri/Info.plist` | `CFBundleDisplayName` / `CFBundleName` = Meetral, mic usage, Utilities category |
| `src-tauri/entitlements.plist` | Audio input, network client, JIT / library validation (notarization) |

## Development

```bash
npm install
npm run tauri dev
```

Grant **Microphone** permission when prompted (required for capture roles).

> Dock hover may show `meetral` under `tauri dev` (Cargo package name). Release `.app` shows **Meetral**.

## Release build (local)

```bash
rustup target add aarch64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin --bundles app,dmg
```

Output:

- `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Meetral.app`
- `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/*.dmg`

Signing / notarization secrets for CI: [release.md](release.md).

## Audio routing (BlackHole)

Use **two** BlackHole drivers so meeting audio and your mic feed do not share one virtual device.

| App role | Device |
|----------|--------|
| Meeting capture | BlackHole **16ch** (input) |
| Teams mic feed | BlackHole **2ch** (output) |
| Local playback | Headphones / built-in output |
| Your microphone | Headset mic or system default |

Teams: **Speaker** → BlackHole **16ch**; **Microphone** → BlackHole **2ch**.

In **Audio MIDI Setup**, set both BlackHole devices to **48 kHz**. Channel count (2 ch or 16 ch) is fine.

Use **Settings → Audio devices → Auto-fill BlackHole** after installing both drivers.

With **Meeting → Direct**, meeting audio is relayed from BlackHole 16ch capture to local playback (Teams speaker no longer goes straight to your headphones).

When Meeting → You is Translated or Clone, Meetral may mix a quiet copy of the meeting floor under TTS. Use headphones and ensure BlackHole/Playback is not looped back into Meeting Capture, or STT will hear the app’s own output. Direct relay is not that mix path — [direct-audio.md](../integrations/direct-audio.md).

## Code signing and notarization

For distribution outside the Mac App Store (Gatekeeper), sign and notarize locally with a **Developer ID Application** certificate:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: …"
export APPLE_ID="…"
export APPLE_PASSWORD="…"   # app-specific password
export APPLE_TEAM_ID="…"
npm run tauri build -- --target aarch64-apple-darwin --bundles app,dmg
```

Unsigned builds are fine for local testing only.

## Logs and config

| Path | Purpose |
|------|---------|
| `~/Library/Application Support/com.meetral.app/config.json` | Settings |
| `~/Library/Application Support/com.meetral.app/logs/app.log` | App log |

## Troubleshooting

| Issue | Fix |
|-------|-----|
| No devices listed | Check Microphone permission in System Settings → Privacy |
| Capture silent | Teams Speaker → BH16; Meeting column **Direct** + LIVE; test speaker in Teams; verify BH16 in QuickTime; refresh device list |
| Device busy / hogged | Close other apps using the same endpoint |
| Gatekeeper blocks install | Sign and notarize the DMG, or staple notarization ticket |
| Dock shows `meetral` | Use a release build; confirm `Info.plist` / `bundleName` |

## QA checklist

Direct/Translate both directions, tray close-to-tray, hot-unplug recovery, Keychain API key persistence, signed + notarized DMG install.
