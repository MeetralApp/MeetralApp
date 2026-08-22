use crate::ai::TranscriptEvent;

pub fn sample_event(direction: &str) -> TranscriptEvent {
    TranscriptEvent {
        direction: direction.into(),
        source_text: Some("hello".into()),
        translated_text: Some("xin chào".into()),
        interim: false,
        turn_complete: true,
        input_segment_finished: true,
        output_segment_finished: true,
        connection_gap: false,
        replace_live: false,
        live_source: None,
        live_translated: None,
    }
}
