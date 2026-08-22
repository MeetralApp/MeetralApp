/** Payload of the `voice-tts-status` event emitted by the Rust voice runtime. */
export type VoiceTtsStatusPayload =
  | { kind: "ready" }
  | { kind: "degraded"; message: string };
