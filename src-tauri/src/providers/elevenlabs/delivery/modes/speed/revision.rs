use std::sync::atomic::{AtomicU64, AtomicU8};

use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::providers::elevenlabs::delivery::core::is_junk_tts_fragment;
use crate::providers::elevenlabs::delivery::state::RelayState;
use crate::providers::elevenlabs::delivery::tts::{
    append_delta, flush_tts, FlushReason, ReconcileAction,
};
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCloneLatencyEvent;

use super::constants::MAJOR_REWRITE_MIN_LCP;

fn longest_common_prefix_chars(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

pub(crate) fn reconcile_after_revision(el_sent: &str, new_text: &str) -> ReconcileAction {
    if el_sent.is_empty() {
        if new_text.trim().is_empty() {
            return ReconcileAction::NoOp;
        }
        return ReconcileAction::AppendSuffix(new_text.to_string());
    }
    let lcp = longest_common_prefix_chars(el_sent, new_text);
    if lcp == 0 {
        return ReconcileAction::MajorRewrite {
            remainder: new_text.to_string(),
        };
    }
    if lcp < MAJOR_REWRITE_MIN_LCP {
        let min_len = el_sent.chars().count().min(new_text.chars().count());
        if min_len >= MAJOR_REWRITE_MIN_LCP {
            return ReconcileAction::MajorRewrite {
                remainder: new_text.to_string(),
            };
        }
    }
    let suffix: String = new_text.chars().skip(lcp).collect();
    if suffix.trim().is_empty() {
        ReconcileAction::NoOp
    } else {
        ReconcileAction::AppendSuffix(suffix)
    }
}

pub(crate) fn apply_pending_revision(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    latency_tx: &Option<mpsc::Sender<VoiceCloneLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
    state: &mut RelayState,
    sentence_flush_at: &mut Option<Instant>,
) {
    let Some(pending) = state.speed.take_pending_revision() else {
        return;
    };

    match reconcile_after_revision(&state.speed.el_sent_anchor, &pending) {
        ReconcileAction::NoOp => {
            state.speed.el_sent_anchor = pending;
        }
        ReconcileAction::AppendSuffix(suffix) => {
            if !is_junk_tts_fragment(&suffix) {
                append_delta(
                    tts_cmd_tx,
                    voice_engine,
                    relay_chars_while_provider,
                    suffix,
                    state,
                );
                *sentence_flush_at = state.speed.arm_sentence_flush(&state.tracker);
            } else {
                state.speed.el_sent_anchor = pending;
            }
        }
        ReconcileAction::MajorRewrite { remainder } => {
            flush_tts(
                tts_cmd_tx,
                voice_engine,
                relay_chars_while_provider,
                latency_tx,
                turn_latency,
                FlushReason::MajorRewrite,
                true,
                state,
            );
            state.speed.major_rewrites += 1;
            state.stamp_phrase_start(turn_latency);
            state.stamp_first_translated(turn_latency);
            if let Some((delta, false)) = state.tracker.append_translated(&remainder) {
                if !is_junk_tts_fragment(&delta) {
                    append_delta(
                        tts_cmd_tx,
                        voice_engine,
                        relay_chars_while_provider,
                        delta,
                        state,
                    );
                    *sentence_flush_at = state.speed.arm_sentence_flush(&state.tracker);
                } else {
                    state.speed.el_sent_anchor = remainder;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcp_reconcile_appends_punctuation_suffix() {
        let action = reconcile_after_revision("Hello world", "Hello, world");
        assert_eq!(action, ReconcileAction::AppendSuffix(", world".to_string()));
    }

    #[test]
    fn lcp_reconcile_major_rewrite_on_unrelated_text() {
        let action = reconcile_after_revision("Hello", "Hi there");
        assert_eq!(
            action,
            ReconcileAction::MajorRewrite {
                remainder: "Hi there".to_string()
            }
        );
    }

    #[test]
    fn lcp_reconcile_noop_when_only_trailing_space_normalized() {
        let action = reconcile_after_revision("Hello world", "Hello world");
        assert_eq!(action, ReconcileAction::NoOp);
    }

    #[test]
    fn lcp_reconcile_appends_exclamation() {
        let action = reconcile_after_revision("Hi there", "Hi there!");
        assert_eq!(action, ReconcileAction::AppendSuffix("!".to_string()));
    }
}
