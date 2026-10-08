# Privacy

Meetral is a desktop app. It has no Meetral account and no Meetral-operated cloud. Nothing is sent to the Meetral project itself. Data leaves your machine only when you turn on a feature that calls a provider you configured, using a key you entered.

There is no product analytics SDK. Logs stay on disk.

## Stays on this device

| Data | Where |
|------|--------|
| Settings | `%APPDATA%\com.meetral.desktop\` on Windows (dev: `com.meetral.desktop.dev`); `~/Library/Application Support/com.meetral.desktop/` on macOS |
| API keys | Encrypted at rest (Windows DPAPI, macOS Keychain). Plaintext keys are not written to `config.json` |
| Meeting library | Local SQLite (`meetings.db`): transcripts, summaries, artifacts |
| Recordings | Ogg Opus files under the app data directory, unless you pick another folder |
| Logs | `logs/app.log` next to the data above |

Direct mode does not call a live translation API. Microphone audio is passed to your virtual meeting device, and meeting audio is passed to local playback.

## Leaves this device

Only while that feature is on, and only to the provider you selected:

| Feature | What is sent | Who receives it |
|---------|----------------|-----------------|
| Translate (live) | Microphone and/or meeting-capture audio | Gemini, OpenAI, or Soniox — whichever engine you selected |
| Engine voice | Audio or translated text, depending on that engine | The same live provider (Gemini/OpenAI speech audio, or Soniox TTS) |
| Custom voice | Translated text for the column you set to Custom | ElevenLabs, Fish Audio, or xAI — per column |
| Summary / artifacts | Transcript text (and the summary prompt) | The summary model you selected: Gemini, OpenAI, or an OpenAI-compatible server you configured |
| Voice preview / key test | A short sample or a key check | The voice or model vendor you are testing |

A custom summary server on `localhost` stays on your machine. A custom server with a public URL receives the transcript.

Keys you paste in **Settings** are sent only to that vendor (as its API credential). They are not sent to Meetral.

Each vendor's own privacy policy covers what they store. Meetral does not control that.

## What you should not expect

- No account, sync, or backup operated by Meetral.
- Closing the window hides the app to the tray; it does not upload a session.
- Overlay text is the same local transcript. It is not a separate upload.
