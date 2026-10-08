# Security

## Reporting a vulnerability

Please use [GitHub private vulnerability reporting](https://github.com/MeetralApp/MeetralApp/security/advisories/new).

Do not open a public issue for bugs that involve API keys, meeting audio, transcripts, recordings, or the installer. Include the OS, app version, and a minimal description. You do not need to send a real API key or a recording.

## Scope

In scope: how keys are stored, how audio and transcripts are handled, and the Windows/macOS installers this repository builds.

Out of scope: vulnerabilities in Gemini, OpenAI, Soniox, ElevenLabs, Fish Audio, xAI, or a custom server you configured. Report those to that vendor.

## Releases

GitHub Release installers are **unsigned**. SmartScreen and Gatekeeper warnings are expected. Download binaries only from this repository's Releases. There is no auto-update feed yet.

Dev signing certificates for Windows sparse package identity (`SPARSE_DEV_PFX_*`) stay in GitHub Actions secrets. Do not commit `.pfx`, `.cer`, or `.env` files.
