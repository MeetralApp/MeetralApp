use std::sync::atomic::{AtomicU64, AtomicU8};

use tokio::sync::mpsc;

use crate::providers::elevenlabs::delivery::core::{
    merge_streaming_text, normalize_interim_translated,
};
use crate::providers::elevenlabs::delivery::state::RelayState;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;

use super::commit::commit_ready_sentences;

pub(crate) fn ingest_sentence_mode(
    translated: &str,
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    turn_latency: &TurnLatencySlot,
    state: &mut RelayState,
) {
    let normalized = normalize_interim_translated(translated);
    state.stamp_first_translated(turn_latency);
    let merged = merge_streaming_text(state.tracker.last_translated(), &normalized);
    state.tracker.set_buffer(&merged);
    commit_ready_sentences(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        state,
        false,
        "sentence_boundary",
    );
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, AtomicU8};

    use tokio::sync::mpsc;

    use super::*;
    use crate::providers::elevenlabs::delivery::modes::natural::commit::commit_ready_sentences;
    use crate::providers::elevenlabs::delivery::state::RelayState;
    use crate::voice::config::VOICE_ENGINE_PROVIDER;
    use crate::voice::shared::latency::TurnLatencySlot;
    use crate::voice::shared::tts_command::TtsTextCommand;

    #[test]
    fn sentence_mode_commits_completed_sentences_only() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let turn = TurnLatencySlot::new_shared();
        let mut state = RelayState::new();

        for frag in ["Xin chào", "mọi người.", "Hôm nay"] {
            ingest_sentence_mode(frag, &tx, &engine, &chars, &turn, &mut state);
        }

        let mut sent = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            sent.push(cmd);
        }
        let appended: Vec<String> = sent
            .iter()
            .filter_map(|c| match c {
                TtsTextCommand::AppendDelta { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        let flushes = sent
            .iter()
            .filter(|c| matches!(c, TtsTextCommand::Flush))
            .count();
        assert_eq!(appended, vec!["Xin chào mọi người.".to_string()]);
        assert_eq!(flushes, 1);
    }

    #[test]
    fn sentence_mode_force_commits_remainder_on_turn_end() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let turn = TurnLatencySlot::new_shared();
        let mut state = RelayState::new();

        ingest_sentence_mode(
            "Một câu chưa kết thúc",
            &tx,
            &engine,
            &chars,
            &turn,
            &mut state,
        );
        assert!(rx.try_recv().is_err());

        commit_ready_sentences(&tx, &engine, &chars, &mut state, true, "turn_complete");

        let mut sent = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            sent.push(cmd);
        }
        let appended: Vec<String> = sent
            .iter()
            .filter_map(|c| match c {
                TtsTextCommand::AppendDelta { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(appended, vec!["Một câu chưa kết thúc".to_string()]);
        assert!(sent.iter().any(|c| matches!(c, TtsTextCommand::Flush)));
    }

    #[test]
    fn sentence_mode_handles_cumulative_openai_buffer() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let turn = TurnLatencySlot::new_shared();
        let mut state = RelayState::new();

        ingest_sentence_mode("Hello there.", &tx, &engine, &chars, &turn, &mut state);
        ingest_sentence_mode("Hello there. How", &tx, &engine, &chars, &turn, &mut state);

        let mut appended = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            if let TtsTextCommand::AppendDelta { text, .. } = cmd {
                appended.push(text);
            }
        }
        assert_eq!(appended, vec!["Hello there.".to_string()]);
    }

    #[test]
    fn sentence_mode_max_chars_commits_at_word_boundary() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let turn = TurnLatencySlot::new_shared();
        let mut state = RelayState::new();

        let long_clause = "alpha ".repeat(40).trim_end().to_string();
        ingest_sentence_mode(&long_clause, &tx, &engine, &chars, &turn, &mut state);

        let mut appended = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            if let TtsTextCommand::AppendDelta { text, .. } = cmd {
                appended.push(text);
            }
        }
        assert!(!appended.is_empty());
        assert!(state.has_natural_pending());
        assert!(state.natural.committed_bytes > 0);
    }
}
