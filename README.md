<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Meetral" width="128" height="128" />
</p>

# Meetral

Desktop app (Windows & macOS) for real-time speech translation in online meetings (Teams, Slack, …). Built with **Tauri 2** + React + Rust.

## Features

- Bidirectional live translate (**You** / **Meeting**): Direct passthrough or Translate
- Multi-provider live engines (**Gemini**, **OpenAI**, **Soniox**)
- Custom voice via **ElevenLabs** and/or **Fish Audio** (You and Meeting columns)
- Meeting library: SQLite transcripts, FTS, AI summary, artifacts, history drawer
- System tray (close-to-tray) and hot-plug audio resilience

## Requirements

| | Windows | macOS |
|--|---------|--------|
| OS | 10/11 x64 | 13+ Apple Silicon |
| Toolchain | [Rust](https://rustup.rs/) + MSVC Build Tools | Xcode CLT + Rust |
| Virtual audio | VoiceMeeter / VB-CABLE / … | [BlackHole 2ch + 16ch](https://existential.audio/blackhole/) — see [docs/development/macos.md](docs/development/macos.md) |

Also: **Node.js 18+**, headphones (reduce echo), and API keys for the providers you use (set in **Settings**).

## Quick start

```bash
git clone <repo-url>
cd MeetralApp
cp .env.example .env   # optional; keys are normally set in Settings
npm install
npm run tauri dev
```

Release build:

```bash
# Windows
npm run tauri build
# → src-tauri/target/release/bundle/msi/

# macOS (Apple Silicon)
npm run tauri build -- --target aarch64-apple-darwin
# → src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/
```

macOS details (permissions, signing): [docs/development/macos.md](docs/development/macos.md).

## Audio roles

App does **not** hardcode device names. In **Settings → Audio devices**, assign four roles:

| Role | Purpose |
|------|---------|
| **Your microphone** | Real mic (empty = system default) |
| **Meeting capture** | Capture meeting audio |
| **Teams mic feed** | Playback into the virtual mic Teams uses |
| **Local playback** | Hear inbound translation (empty = system default) |

### Windows (e.g. VoiceMeeter Banana)

| App role | Example device |
|----------|----------------|
| Meeting capture | Out B2 |
| Teams mic feed | AUX Input |
| Local playback | VAIO Input (or headphones on A1) |
| Your microphone | Headset mic (system default) |

Teams: **Speaker** → VAIO Input; **Microphone** → Out B1. See in-app **Settings → Example: virtual audio mixer setup**.

### macOS (BlackHole)

| App role | Device |
|----------|--------|
| Meeting capture | BlackHole **16ch** (input) |
| Teams mic feed | BlackHole **2ch** (output) |
| Local playback | Headphones / Mac speakers |
| Your microphone | Mac mic or headset (system default) |

Teams: **Speaker** → BlackHole **16ch**; **Microphone** → BlackHole **2ch**. In **Audio MIDI Setup**, set both BlackHole devices to **48 kHz**. After install, use **Settings → Audio devices → Auto-fill BlackHole**. More: [docs/development/macos.md](docs/development/macos.md).

## Usage

1. Route virtual audio + Teams/Slack, then pick the four roles → **Save**.
2. Configure **Live translation**, **Voice**, and **Summary** in Settings.
3. Per column (**You** / **Meeting**): **Direct** (no API) or **Translate**.
4. When Translate is on: **Translated** / **Original** / **Text**.

Closing the window hides to the **system tray** by default (relay keeps running). Use tray **Quit** to exit fully.

## Development

```bash
npm run tauri dev      # app
npm test               # frontend (Vitest)
cd src-tauri && cargo test --lib   # backend
npm run test:all       # both
```

Agent entrypoint: [AGENTS.md](AGENTS.md). Docs map: [docs/README.md](docs/README.md). Tests: [docs/development/testing.md](docs/development/testing.md).

## Structure

```
src/                 React (Vite) — `src/features/*` + `src/shared/*`
src-tauri/           Rust (Tauri 2) — one crate
design-system/       UI tokens and per-screen rules
docs/                Project knowledge (architecture, features, integrations, development)
AGENTS.md            Instructions for coding agents
```

Details: [docs/README.md](docs/README.md). Do not put implementation contracts in this README.

## Docs

| Doc | Topic |
|-----|--------|
| [AGENTS.md](AGENTS.md) | Agent instructions |
| [docs/README.md](docs/README.md) | Documentation map |
| [docs/features/current.md](docs/features/current.md) | Version, out of scope, deferred |
| [docs/features/catalog.md](docs/features/catalog.md) | Shipped feature catalog |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module boundaries |
| [docs/development/testing.md](docs/development/testing.md) | Unit test standards |
| [docs/development/windows.md](docs/development/windows.md) | Sparse identity / os 15700 |
| [docs/integrations/direct-audio.md](docs/integrations/direct-audio.md) | Direct audio relay |
| [docs/development/macos.md](docs/development/macos.md) | macOS build & audio |
| [design-system/MASTER.md](design-system/MASTER.md) | UI design system |

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `The process has no package identity` (os error 15700) on `tauri dev` / `cargo run` | See [docs/development/windows.md](docs/development/windows.md). Unregister leftover Meetral Appx, then retry. |
| Missing audio role banner | Settings → Audio devices → assign → **Save** → **Refresh** |
| Device not found / start fails | Confirm endpoints still exist in OS sound settings; **Refresh** |
| Echo | Use headphones |
| Missing API key | Settings → enter key → **Save** |
| High latency | Often ~300ms–1s depending on provider |
| Debug log (Windows) | `%APPDATA%\com.meetral.desktop\logs\app.log` |
| Debug log (macOS) | `~/Library/Application Support/com.meetral.desktop/logs/app.log` |
| macOS mic permission | System Settings → Privacy → Microphone; grant when prompted |
