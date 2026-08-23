use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::types::VoiceCustomLatencyEvent;

/// Per-turn latency timestamps shared between transcript delivery and TTS workers.
#[derive(Debug, Default)]
pub struct TurnLatencySlot {
    phrase_started_ms: AtomicU64,
    first_translated_ms: AtomicU64,
    first_tts_audio_ms: AtomicU64,
}

impl TurnLatencySlot {
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn begin_turn(&self, phrase_started_ms: u64, first_translated_ms: u64) {
        self.phrase_started_ms
            .store(phrase_started_ms, Ordering::SeqCst);
        self.first_translated_ms
            .store(first_translated_ms, Ordering::SeqCst);
        self.first_tts_audio_ms.store(0, Ordering::SeqCst);
    }

    pub fn record_first_audio(&self) {
        let now = crate::audio::monotonic_ms();
        let _ =
            self.first_tts_audio_ms
                .compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst);
    }

    pub fn build_flush_event(
        &self,
        flush_ms: u64,
        flush_reason: &str,
    ) -> Option<VoiceCustomLatencyEvent> {
        let phrase_started = self.phrase_started_ms.load(Ordering::SeqCst);
        let first_translated = self.first_translated_ms.load(Ordering::SeqCst);
        if phrase_started == 0 || first_translated == 0 {
            return None;
        }
        let first_audio = self.first_tts_audio_ms.load(Ordering::SeqCst);
        let translate_ms = first_translated.saturating_sub(phrase_started) as i64;
        let tts_ms = if first_audio > 0 {
            first_audio.saturating_sub(first_translated) as i64
        } else {
            0
        };
        let total_ms = if first_audio > 0 {
            first_audio.saturating_sub(phrase_started) as i64
        } else {
            flush_ms.saturating_sub(phrase_started) as i64
        };
        Some(VoiceCustomLatencyEvent {
            phrase_started_ms: phrase_started,
            first_translated_text_ms: first_translated,
            first_tts_audio_ms: first_audio,
            flush_ms,
            flush_reason: flush_reason.to_string(),
            translate_ms,
            tts_ms,
            total_ms,
        })
    }

    pub fn reset(&self) {
        self.phrase_started_ms.store(0, Ordering::SeqCst);
        self.first_translated_ms.store(0, Ordering::SeqCst);
        self.first_tts_audio_ms.store(0, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn seed_first_audio_ms(&self, ms: u64) {
        self.first_tts_audio_ms.store(ms, Ordering::SeqCst);
    }
}
