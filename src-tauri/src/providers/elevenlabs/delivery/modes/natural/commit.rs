use std::sync::atomic::{AtomicU64, AtomicU8};

use crate::providers::elevenlabs::delivery::core::{
    is_junk_tts_fragment, last_committable_sentence_end, last_word_boundary,
};
use crate::providers::elevenlabs::delivery::state::RelayState;
use crate::providers::elevenlabs::delivery::tts::commit_sentence_chunk;
use crate::voice::shared::tts_command::TtsTextCommand;
use tokio::sync::mpsc;

use super::constants::SENTENCE_MAX_CHARS;

pub(crate) fn commit_ready_sentences(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    state: &mut RelayState,
    force_all: bool,
    force_reason: &'static str,
) {
    loop {
        let full_len = state.tracker.last_translated().len();
        if state.natural.committed_bytes >= full_len {
            break;
        }
        let remainder =
            state.tracker.last_translated()[state.natural.committed_bytes..].to_string();
        let (cut, flush_reason) = if force_all {
            (remainder.len(), force_reason)
        } else if let Some(end) = last_committable_sentence_end(&remainder) {
            (end, "sentence_boundary")
        } else if remainder.chars().count() >= SENTENCE_MAX_CHARS {
            (
                last_word_boundary(&remainder).unwrap_or(remainder.len()),
                "sentence_max_chars",
            )
        } else {
            break;
        };

        let chunk = remainder[..cut].trim().to_string();
        state.natural.committed_bytes += cut;
        if !chunk.is_empty() && !is_junk_tts_fragment(&chunk) {
            commit_sentence_chunk(
                tts_cmd_tx,
                voice_engine,
                relay_chars_while_provider,
                &chunk,
                flush_reason,
                state,
            );
        }

        if force_all {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::elevenlabs::delivery::state::RelayState;
    use crate::voice::config::VOICE_ENGINE_PROVIDER;
    use crate::voice::shared::tts_command::TtsTextCommand;

    #[test]
    fn natural_idle_commits_pending_tail() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let mut state = RelayState::new();

        state.tracker.set_buffer("Một câu chưa kết thúc đủ dài");
        commit_ready_sentences(&tx, &engine, &chars, &mut state, true, "sentence_idle");

        let mut appended = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            if let TtsTextCommand::AppendDelta { text, .. } = cmd {
                appended.push(text);
            }
        }
        assert_eq!(appended, vec!["Một câu chưa kết thúc đủ dài".to_string()]);
        assert_eq!(
            state.natural.committed_bytes,
            state.tracker.last_translated().len()
        );
    }
}
