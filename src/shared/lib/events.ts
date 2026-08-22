/**
* Tauri event names — single source of truth for FE listeners.
* The backend emits these from Rust; keep in sync with `app.emit(...)` sites.
*/
export const APP_EVENTS = {
  appState: "app-state",
  transcript: "transcript",
  segmentPreview: "segment-preview",
  segmentCommitted: "segment-committed",
  meetingChanged: "meeting-changed",
  meetingUpdated: "meeting-updated",
  voiceTtsStatus: "voice-tts-status",
  summaryProgress: "summary-progress",
  summaryDone: "summary-done",
  summaryError: "summary-error",
  overlayPassThrough: "overlay-pass-through",
  overlayPreviewMode: "overlay-preview-mode",
  inboundTtsQueueDrop: "inbound-tts-queue-drop",
} as const;
