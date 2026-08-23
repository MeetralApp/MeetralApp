use serde::{Deserialize, Serialize};

const ELEVENLABS_VOICES_URL: &str = "https://api.elevenlabs.io/v2/voices";
const ELEVENLABS_VOICES_V1_URL: &str = "https://api.elevenlabs.io/v1/voices";
const ELEVENLABS_MODELS_URL: &str = "https://api.elevenlabs.io/v1/models";
const MAX_PAGES: u32 = 5;
/// Candidate pool only. `saved` still includes owned PVC/clone voices that were
/// removed from **My Voices**; we narrow to rows the dashboard treats as saved.
const VOICES_CANDIDATE_TYPE: &str = "saved";
/// Catalog / validate HTTP timeout (list models, voices, voice GET).
const VOICE_HTTP_TIMEOUT_SECS: u64 = 15;
/// TTS preview POST can be slower than catalog GETs.
const VOICE_PREVIEW_HTTP_TIMEOUT_SECS: u64 = 30;

fn voice_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("build http client: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElevenLabsVoiceOption {
    pub voice_id: String,
    pub name: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElevenLabsModelOption {
    pub model_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub clone_stream_supported: bool,
    pub requires_alpha_access: bool,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    model_id: String,
    name: String,
    description: Option<String>,
    can_do_text_to_speech: Option<bool>,
    requires_alpha_access: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VoicesV1Response {
    voices: Vec<VoiceItemV1>,
}

#[derive(Debug, Deserialize)]
struct VoiceItemV1 {
    voice_id: String,
}

#[derive(Debug, Deserialize)]
struct VoicesV2Response {
    voices: Vec<VoiceItem>,
    has_more: Option<bool>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoiceItem {
    voice_id: String,
    name: String,
    category: Option<String>,
    preview_url: Option<String>,
    collection_ids: Option<Vec<String>>,
    favorited_at_unix: Option<i64>,
    is_bookmarked: Option<bool>,
}

/// Aligns with the ElevenLabs **My Voices** tab.
///
/// `voice_type=saved` alone is wider than that tab: it keeps account-owned PVC
/// even after removal from My Voices. Active entries are favorited/bookmarked or
/// in a collection; owned instant clones in `saved` are kept via category.
fn is_in_my_voices(voice: &VoiceItem) -> bool {
    if voice
        .collection_ids
        .as_ref()
        .is_some_and(|ids| !ids.is_empty())
    {
        return true;
    }
    if voice.favorited_at_unix.is_some() {
        return true;
    }
    if voice.is_bookmarked == Some(true) {
        return true;
    }
    voice.category.as_deref() == Some("cloned")
}

pub async fn list_voices(api_key: &str) -> Result<Vec<ElevenLabsVoiceOption>, String> {
    let usable_ids = fetch_v1_voice_ids(api_key).await.ok();
    let candidates = fetch_voices_v2(api_key, Some(VOICES_CANDIDATE_TYPE)).await?;
    let mut all: Vec<ElevenLabsVoiceOption> = candidates
        .into_iter()
        .filter(is_in_my_voices)
        .filter(|voice| {
            usable_ids
                .as_ref()
                .map(|ids| ids.contains(&voice.voice_id))
                .unwrap_or(true)
        })
        .map(into_voice_option)
        .collect();

    if all.is_empty() {
        let personal = fetch_voices_v2(api_key, Some("personal")).await?;
        all = personal
            .into_iter()
            .filter(is_in_my_voices)
            .filter(|voice| {
                usable_ids
                    .as_ref()
                    .map(|ids| ids.contains(&voice.voice_id))
                    .unwrap_or(true)
            })
            .map(into_voice_option)
            .collect();
    }

    all.sort_by_key(|a| a.name.to_lowercase());
    Ok(all)
}

pub async fn list_models(api_key: &str) -> Result<Vec<ElevenLabsModelOption>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let response = client
        .get(ELEVENLABS_MODELS_URL)
        .header("xi-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format_elevenlabs_error(&body));
    }

    let raw = response
        .text()
        .await
        .map_err(|e| format!("Invalid ElevenLabs models response: {e}"))?;
    let items = parse_models_response(&raw)?;

    let mut models: Vec<ElevenLabsModelOption> = items
        .into_iter()
        .filter(|model| model.can_do_text_to_speech.unwrap_or(false))
        .map(into_model_option)
        .collect();

    sort_model_options(&mut models);
    Ok(models)
}

fn parse_models_response(raw: &str) -> Result<Vec<ModelItem>, String> {
    if let Ok(models) = serde_json::from_str::<Vec<ModelItem>>(raw) {
        return Ok(models);
    }
    #[derive(Debug, Deserialize)]
    struct ModelsEnvelope {
        models: Vec<ModelItem>,
    }
    serde_json::from_str::<ModelsEnvelope>(raw)
        .map(|envelope| envelope.models)
        .map_err(|e| format!("Invalid ElevenLabs models JSON: {e}"))
}

fn into_model_option(model: ModelItem) -> ElevenLabsModelOption {
    let clone_stream_supported = super::config::supports_clone_stream_input(&model.model_id);
    ElevenLabsModelOption {
        model_id: model.model_id,
        name: model.name,
        description: model.description,
        clone_stream_supported,
        requires_alpha_access: model.requires_alpha_access.unwrap_or(false),
    }
}

fn sort_model_options(models: &mut [ElevenLabsModelOption]) {
    models.sort_by(|a, b| {
        b.clone_stream_supported
            .cmp(&a.clone_stream_supported)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

pub async fn validate_voice(api_key: &str, voice_id: &str) -> Result<(), String> {
    let voice_id = voice_id.trim();
    if voice_id.is_empty() {
        return Err("ElevenLabs voice ID is required.".into());
    }
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("https://api.elevenlabs.io/v1/voices/{voice_id}");
    let response = client
        .get(url)
        .header("xi-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if response.status().is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(super::protocol::user_message_for_elevenlabs_error(
        &format_elevenlabs_error(&body),
    ))
}

const PREVIEW_SAMPLE_TEXT: &str = "Hello, this is a voice test.";

pub async fn preview_voice(
    api_key: &str,
    voice_id: &str,
    config: &crate::config::AppConfig,
    devices: &[crate::audio::AudioDeviceInfo],
) -> Result<(), String> {
    validate_voice(api_key, voice_id).await?;

    let client = voice_http_client(VOICE_PREVIEW_HTTP_TIMEOUT_SECS)?;
    let mut url =
        format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}?output_format=pcm_24000");
    if let Some(code) = config.resolve_elevenlabs_tts_language_code() {
        url.push_str(&format!("&language_code={code}"));
    }
    let body = serde_json::json!({
        "text": PREVIEW_SAMPLE_TEXT,
        "model_id": config.elevenlabs.elevenlabs_tts_model,
        "voice_settings": {
            "stability": config.elevenlabs.elevenlabs_stability,
            "similarity_boost": config.elevenlabs.elevenlabs_similarity_boost,
            "speed": super::config::elevenlabs_speed_for_api(config.elevenlabs.elevenlabs_speed),
            "use_speaker_boost": config.elevenlabs.elevenlabs_use_speaker_boost,
        }
    });
    let response = client
        .post(url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Preview request failed: {e}"))?;

    if !response.status().is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(super::protocol::user_message_for_elevenlabs_error(
            &format_elevenlabs_error(&err_body),
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Preview read failed: {e}"))?;
    if bytes.len() < 2 || !bytes.len().is_multiple_of(2) {
        return Err("Preview returned invalid audio".into());
    }
    let pcm: Vec<i16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| i16::from_le_bytes(*chunk))
        .collect();
    let pcm_48k = crate::audio::resampler::upsample_24k_to_48k(&pcm)
        .map_err(|e| format!("Preview resample failed: {e}"))?;

    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
    let _ = tx.try_send(pcm_48k);
    drop(tx);

    let playback = crate::audio::start_playback_for_role(
        crate::audio::AudioRole::LocalPlayback,
        config,
        devices,
        rx,
    )
    .map_err(|e| format!("Preview playback failed: {e}"))?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    playback.stop();
    Ok(())
}

async fn fetch_v1_voice_ids(api_key: &str) -> Result<std::collections::HashSet<String>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let response = client
        .get(ELEVENLABS_VOICES_V1_URL)
        .header("xi-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format_elevenlabs_error(&body));
    }
    let page: VoicesV1Response = response
        .json()
        .await
        .map_err(|e| format!("Invalid ElevenLabs v1 response: {e}"))?;
    Ok(page.voices.into_iter().map(|v| v.voice_id).collect())
}

fn into_voice_option(voice: VoiceItem) -> ElevenLabsVoiceOption {
    ElevenLabsVoiceOption {
        voice_id: voice.voice_id,
        name: voice.name,
        category: voice.category.unwrap_or_else(|| "unknown".into()),
        preview_url: voice.preview_url,
    }
}

async fn fetch_voices_v2(
    api_key: &str,
    voice_type: Option<&str>,
) -> Result<Vec<VoiceItem>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let mut all = Vec::new();
    let mut next_token: Option<String> = None;

    for _ in 0..MAX_PAGES {
        let mut url = reqwest::Url::parse(ELEVENLABS_VOICES_URL)
            .map_err(|e| format!("Invalid voices URL: {e}"))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("page_size", "100");
            if let Some(filter) = voice_type {
                pairs.append_pair("voice_type", filter);
            }
            pairs.append_pair("sort", "name");
            pairs.append_pair("sort_direction", "asc");
            if let Some(ref token) = next_token {
                pairs.append_pair("next_page_token", token);
            }
        }

        let response = client
            .get(url)
            .header("xi-api-key", api_key)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format_elevenlabs_error(&body));
        }

        let page: VoicesV2Response = response
            .json()
            .await
            .map_err(|e| format!("Invalid ElevenLabs response: {e}"))?;

        all.extend(page.voices);

        let has_more = page.has_more.unwrap_or(false);
        if has_more {
            if let Some(token) = page.next_page_token.filter(|t| !t.is_empty()) {
                next_token = Some(token);
                continue;
            }
        }
        break;
    }

    Ok(all)
}

fn format_elevenlabs_error(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(detail) = value.get("detail") {
            if let Some(msg) = detail.get("message").and_then(|v| v.as_str()) {
                return format!("ElevenLabs API error: {msg}");
            }
            if let Some(status) = detail.get("status").and_then(|v| v.as_str()) {
                return format!("ElevenLabs API error: {status}");
            }
        }
    }
    format!("ElevenLabs API error: {body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_model_options_puts_stream_supported_first() {
        let mut models = vec![
            ElevenLabsModelOption {
                model_id: "eleven_v3".into(),
                name: "Eleven v3".into(),
                description: None,
                clone_stream_supported: false,
                requires_alpha_access: false,
            },
            ElevenLabsModelOption {
                model_id: "eleven_flash_v2_5".into(),
                name: "Flash v2.5".into(),
                description: None,
                clone_stream_supported: true,
                requires_alpha_access: false,
            },
            ElevenLabsModelOption {
                model_id: "eleven_turbo_v2".into(),
                name: "Turbo v2".into(),
                description: None,
                clone_stream_supported: true,
                requires_alpha_access: false,
            },
        ];
        sort_model_options(&mut models);
        assert_eq!(models[0].model_id, "eleven_flash_v2_5");
        assert_eq!(models[1].model_id, "eleven_turbo_v2");
        assert_eq!(models[2].model_id, "eleven_v3");
    }

    #[test]
    fn into_model_option_marks_stream_supported_ids() {
        let flash = into_model_option(ModelItem {
            model_id: "eleven_flash_v2_5".into(),
            name: "Flash".into(),
            description: None,
            can_do_text_to_speech: Some(true),
            requires_alpha_access: None,
        });
        assert!(flash.clone_stream_supported);

        let v3 = into_model_option(ModelItem {
            model_id: "eleven_v3".into(),
            name: "v3".into(),
            description: None,
            can_do_text_to_speech: Some(true),
            requires_alpha_access: None,
        });
        assert!(!v3.clone_stream_supported);
    }

    #[test]
    fn format_error_reads_detail_message() {
        let body = r#"{"detail":{"status":"invalid_api_key","message":"Invalid API key"}}"#;
        assert!(format_elevenlabs_error(body).contains("Invalid API key"));
    }

    #[test]
    fn is_in_my_voices_uses_favorite_bookmark_or_collection() {
        let favorited = VoiceItem {
            voice_id: "a".into(),
            name: "Nhan".into(),
            category: Some("cloned".into()),
            preview_url: None,
            collection_ids: None,
            favorited_at_unix: Some(1_710_000_000),
            is_bookmarked: None,
        };
        assert!(is_in_my_voices(&favorited));

        let bookmarked = VoiceItem {
            voice_id: "b".into(),
            name: "Adam".into(),
            category: Some("professional".into()),
            preview_url: None,
            collection_ids: None,
            favorited_at_unix: None,
            is_bookmarked: Some(true),
        };
        assert!(is_in_my_voices(&bookmarked));

        let removed_from_my_voices = VoiceItem {
            voice_id: "c".into(),
            name: "Ivy".into(),
            category: Some("professional".into()),
            preview_url: None,
            collection_ids: None,
            favorited_at_unix: None,
            is_bookmarked: None,
        };
        assert!(!is_in_my_voices(&removed_from_my_voices));

        let owned_clone = VoiceItem {
            voice_id: "d".into(),
            name: "Nhan".into(),
            category: Some("cloned".into()),
            preview_url: None,
            collection_ids: None,
            favorited_at_unix: None,
            is_bookmarked: None,
        };
        assert!(is_in_my_voices(&owned_clone));
    }
}
