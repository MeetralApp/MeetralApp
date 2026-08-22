use anyhow::Result;
use tauri::{AppHandle, Emitter};

use crate::ai::SummaryLlmClient;

use super::models::SummaryProgressEvent;
use super::models::TranscriptSegmentView;
use super::prompts::{
    build_chunk_extract_prompt, build_merge_summary_prompt, build_summary_prompt,
    is_known_summary_template, render_summary_context_block, resolve_template, PromptContext,
    CHUNK_CONTEXT_CHAR_BUDGET, SUMMARY_CONTEXT_CHAR_BUDGET,
};
use super::store::MeetingStore;
use super::summary_doc::{doc_to_prose, seed_from_brief};
use super::summary_schema::parse_summary_json;

const SUMMARY_CHUNK_SIZE: i64 = 100;
const SUMMARY_MAX_SEGMENTS: i64 = 10_000;
const SUMMARY_LLM_MAX_ATTEMPTS: u32 = 2;

pub async fn generate_meeting_summary(
    store: &MeetingStore,
    meeting_id: &str,
    client: &SummaryLlmClient,
    summary_model: &str,
    template_id: &str,
    summary_language: &str,
    meeting_context: &crate::config::MeetingContextPayload,
    app: Option<&AppHandle>,
    gen_registry: Option<&super::summary_gen_registry::SummaryGenerationRegistry>,
) -> Result<String> {
    if !is_known_summary_template(template_id) {
        anyhow::bail!("Unknown summary template: {template_id}");
    }
    if !client.supports_summary_language(summary_language) {
        anyhow::bail!("Unsupported summary language: {summary_language}");
    }
    let prompt_id = resolve_template(template_id)
        .map(|spec| spec.prompt_id())
        .unwrap_or_else(|| format!("{template_id}@v?"));

    let meeting = store.get_meeting(meeting_id, false)?;
    if meeting.status == super::models::MeetingStatus::Live {
        anyhow::bail!("End the meeting before generating a summary");
    }

    let segments = load_segments_for_summary(store, meeting_id)?;
    if segments.is_empty() {
        anyhow::bail!("No transcript segments to summarize");
    }

    let context_block = render_summary_context_block(meeting_context, SUMMARY_CONTEXT_CHAR_BUDGET);
    // Map-step chunk prompt repeats context per call — smaller budget.
    let chunk_context_block =
        render_summary_context_block(meeting_context, CHUNK_CONTEXT_CHAR_BUDGET);
    let prompt_ctx = PromptContext {
        summary_language,
        my_language: &meeting.my_language,
        meeting_language: &meeting.meeting_language,
        transcript: "",
        session_mode: meeting.session_mode.as_str(),
        meeting_title: &meeting.title,
        context_block: context_block.as_deref(),
    };

    crate::debug_log!(
        meeting_id,
        provider = %client.origin_label(),
        model = summary_model,
        template_id,
        prompt_id = prompt_id.as_str(),
        summary_language,
        segments = segments.len(),
        strategy = if segments.len() as i64 <= SUMMARY_CHUNK_SIZE {
            "single"
        } else {
            "incremental"
        },
        "summary start"
    );

    let response_text = if segments.len() as i64 <= SUMMARY_CHUNK_SIZE {
        let transcript = format_transcript(&segments);
        let prompt = build_summary_prompt(
            template_id,
            &PromptContext {
                transcript: &transcript,
                ..prompt_ctx
            },
        )?;
        emit_summary_progress(app, gen_registry, meeting_id, "generate", 1, 1, None);
        crate::debug_log!(meeting_id, phase = "generate", "summary call");
        call_summary_with_retry(client, summary_model, &prompt, true).await?
    } else {
        // Streaming incremental merge: seed from chunk 0,
        // then extract+merge each subsequent chunk so FE gets partial text
        // early instead of waiting for a final map-reduce merge.
        let chunks = chunk_segments(&segments, SUMMARY_CHUNK_SIZE as usize);
        let total = chunks.len() as u32;
        crate::debug_log!(
            meeting_id,
            chunk_count = chunks.len(),
            total_phases = total,
            "summary incremental"
        );

        emit_summary_progress(app, gen_registry, meeting_id, "chunk", 1, total, None);
        let seed_transcript = format_transcript(&chunks[0]);
        let seed_prompt = build_summary_prompt(
            template_id,
            &PromptContext {
                transcript: &seed_transcript,
                ..prompt_ctx
            },
        )?;
        let mut current_summary =
            call_summary_with_retry(client, summary_model, &seed_prompt, true).await?;
        emit_summary_progress(
            app,
            gen_registry,
            meeting_id,
            "partial",
            1,
            total,
            Some(truncate_partial(&current_summary, 1_200)),
        );

        for (index, chunk) in chunks.iter().enumerate().skip(1) {
            let current_idx = index as u32 + 1;
            emit_summary_progress(
                app,
                gen_registry,
                meeting_id,
                "chunk",
                current_idx,
                total,
                None,
            );
            let chunk_transcript = format_transcript(chunk);
            let chunk_prompt = build_chunk_extract_prompt(
                &PromptContext {
                    context_block: chunk_context_block.as_deref(),
                    ..prompt_ctx
                },
                &chunk_transcript,
            );
            let chunk_notes =
                call_summary_with_retry(client, summary_model, &chunk_prompt, false).await?;
            let chunk_notes = strip_no_notes(&chunk_notes);
            if chunk_notes.is_empty() {
                continue;
            }
            let merge_notes = format!(
                "Previous summary (update in place; keep structure):\n{}\n\nNew notes to merge:\n{}",
                current_summary, chunk_notes
            );
            let merge_prompt = build_merge_summary_prompt(template_id, &prompt_ctx, &merge_notes)?;
            current_summary =
                call_summary_with_retry(client, summary_model, &merge_prompt, true).await?;
            emit_summary_progress(
                app,
                gen_registry,
                meeting_id,
                "partial",
                current_idx,
                total,
                Some(truncate_partial(&current_summary, 1_200)),
            );
        }
        current_summary
    };

    let json_text = extract_json_block(&response_text);
    let parsed = parse_summary_json(template_id, json_text)?;
    let display_text = persist_summary_and_artifacts(
        store,
        meeting_id,
        template_id,
        summary_language,
        &parsed,
        &segments,
    )?;
    let artifact_count = store
        .list_artifacts(meeting_id, None, None)
        .map(|a| a.len())
        .unwrap_or(0);
    crate::debug_log!(
        meeting_id,
        display_chars = display_text.len(),
        artifacts = artifact_count,
        "summary done"
    );
    Ok(display_text)
}

/// Persist the TipTap summary doc, then derive structured artifacts from the
/// same parsed brief (no extra LLM call). Artifact failures must not fail the
/// summary — artifacts are derived data and regenerate on the next run.
fn persist_summary_and_artifacts(
    store: &MeetingStore,
    meeting_id: &str,
    template_id: &str,
    summary_language: &str,
    parsed: &super::summary_schema::MeetingBriefSummary,
    segments: &[TranscriptSegmentView],
) -> Result<String> {
    let doc = seed_from_brief(parsed, segments, summary_language);
    let generated_json = serde_json::to_string(&doc)?;
    let display_text = doc_to_prose(&doc);

    store.save_summary(meeting_id, template_id, summary_language, &generated_json)?;

    if let Err(err) = store.replace_meeting_artifacts(meeting_id, parsed, super::store::wall_ms()) {
        tracing::warn!(meeting_id = %meeting_id, error = %err, "artifact replace failed after summary save");
    }

    Ok(display_text)
}

fn load_segments_for_summary(
    store: &MeetingStore,
    meeting_id: &str,
) -> Result<Vec<TranscriptSegmentView>> {
    let total = store.count_segments(meeting_id)?;
    if total == 0 {
        return Ok(Vec::new());
    }
    let cap = total.min(SUMMARY_MAX_SEGMENTS);
    let mut segments = Vec::with_capacity(cap as usize);
    let mut offset = 0i64;
    while (segments.len() as i64) < cap {
        let page =
            store.list_segments_chronological_page(meeting_id, offset, SUMMARY_CHUNK_SIZE)?;
        if page.is_empty() {
            break;
        }
        offset += page.len() as i64;
        segments.extend(page);
    }
    segments.truncate(cap as usize);
    Ok(segments)
}

fn chunk_segments(
    segments: &[TranscriptSegmentView],
    chunk_size: usize,
) -> Vec<Vec<TranscriptSegmentView>> {
    segments
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

fn format_transcript(segments: &[TranscriptSegmentView]) -> String {
    let mut transcript = String::new();
    for seg in segments {
        let speaker = if seg.direction == "outbound" {
            "You"
        } else {
            "Meeting"
        };
        let text = if seg.translated_text.trim().is_empty() {
            seg.source_text.as_str()
        } else {
            seg.translated_text.as_str()
        };
        transcript.push_str(&format!(
            "[{direction} #{sequence}] ({speaker}) {text}\n",
            direction = seg.direction,
            sequence = seg.sequence,
            speaker = speaker,
            text = text,
        ));
    }
    transcript
}

fn truncate_partial(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let end = trimmed
        .char_indices()
        .nth(max_chars.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(trimmed.len());
    format!("{}…", &trimmed[..end])
}

fn emit_summary_progress(
    app: Option<&AppHandle>,
    gen_registry: Option<&super::summary_gen_registry::SummaryGenerationRegistry>,
    meeting_id: &str,
    phase: &str,
    current: u32,
    total: u32,
    partial_text: Option<String>,
) {
    if let Some(registry) = gen_registry {
        registry.progress(meeting_id, phase, current, total);
    }
    if let Some(app) = app {
        let _ = app.emit(
            "summary-progress",
            SummaryProgressEvent {
                meeting_id: meeting_id.to_string(),
                phase: phase.to_string(),
                current,
                total,
                partial_text,
            },
        );
    }
}

async fn call_summary_with_retry(
    client: &SummaryLlmClient,
    model: &str,
    prompt: &str,
    json_response: bool,
) -> Result<String> {
    client
        .generate_with_retry(
            model,
            prompt,
            json_response,
            Some(super::prompts::SYSTEM_INSTRUCTION),
            SUMMARY_LLM_MAX_ATTEMPTS,
        )
        .await
}

fn extract_json_block(text: &str) -> &str {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        return inner;
    }
    trimmed
}

/// Drop the `(no notes)` sentinel line(s) that `chunk.txt` instructs the model to
/// emit for an empty chunk, so empty chunk output never leaks into the merge step.
fn strip_no_notes(notes: &str) -> &str {
    let trimmed = notes.trim();
    if trimmed == "(no notes)" {
        return "";
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::models::MeetingStatus;
    use crate::meeting::prompts::DEFAULT_SUMMARY_TEMPLATE_ID;
    use crate::meeting::summary_schema::{
        MeetingBriefSummary, SegmentRef, SummaryEntity, SummaryPoint, SummaryPointBlock,
    };

    fn brief_with(
        decisions: Vec<SummaryPoint>,
        entities: Vec<SummaryEntity>,
    ) -> MeetingBriefSummary {
        MeetingBriefSummary {
            schema_version: 3,
            overview: SummaryPointBlock::default(),
            key_points: SummaryPointBlock::default(),
            decisions: SummaryPointBlock { points: decisions },
            action_items: SummaryPointBlock::default(),
            open_questions: SummaryPointBlock::default(),
            entities,
            yesterday: SummaryPointBlock::default(),
            today: SummaryPointBlock::default(),
            blockers: SummaryPointBlock::default(),
        }
    }

    #[test]
    fn persist_summary_writes_artifacts_then_replaces_cleanly() {
        let store = MeetingStore::open_in_memory().unwrap();
        let meeting = store
            .create_meeting("M", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();

        let first = brief_with(
            vec![SummaryPoint {
                id: "de-1".into(),
                text: "Ship v1".into(),
                owner: None,
                due: None,
                segment_refs: vec![SegmentRef {
                    direction: "outbound".into(),
                    sequence: 1,
                }],
                supersedes_index: None,
            }],
            vec![SummaryEntity {
                name: "AUTH-123".into(),
                kind: "ticket".into(),
                segment_refs: Vec::new(),
            }],
        );
        persist_summary_and_artifacts(
            &store,
            &meeting.id,
            DEFAULT_SUMMARY_TEMPLATE_ID,
            "en",
            &first,
            &[],
        )
        .unwrap();
        assert_eq!(
            store.list_artifacts(&meeting.id, None, None).unwrap().len(),
            1
        );
        assert_eq!(store.list_entities(&meeting.id).unwrap().len(), 1);

        // Regenerate with an empty brief: artifacts/entities wipe clean.
        persist_summary_and_artifacts(
            &store,
            &meeting.id,
            DEFAULT_SUMMARY_TEMPLATE_ID,
            "en",
            &brief_with(Vec::new(), Vec::new()),
            &[],
        )
        .unwrap();
        assert!(store
            .list_artifacts(&meeting.id, None, None)
            .unwrap()
            .is_empty());
        assert!(store.list_entities(&meeting.id).unwrap().is_empty());
    }

    #[test]
    fn chunk_segments_splits_evenly() {
        let segments: Vec<TranscriptSegmentView> = (0..5)
            .map(|i| TranscriptSegmentView {
                id: format!("s{i}"),
                meeting_id: "m1".into(),
                direction: "outbound".into(),
                sequence: i,
                source_text: "a".into(),
                translated_text: "b".into(),
                started_at_ms: i as i64,
                ended_at_ms: i as i64,
                connection_gap: false,
            })
            .collect();
        let chunks = chunk_segments(&segments, 2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[2].len(), 1);
    }

    #[test]
    fn strip_no_notes_removes_sentinel_only() {
        assert_eq!(strip_no_notes("(no notes)"), "");
        assert_eq!(strip_no_notes("  (no notes)\n"), "");
        // Real notes pass through, including one that starts with the phrase
        // but contains actual content (must not be a false-positive strip).
        assert_eq!(
            strip_no_notes("- Draft the Q3 budget [outbound #1]"),
            "- Draft the Q3 budget [outbound #1]"
        );
        assert_eq!(
            strip_no_notes("(no notes)\n- real note [inbound #2]"),
            "(no notes)\n- real note [inbound #2]"
        );
    }
}
