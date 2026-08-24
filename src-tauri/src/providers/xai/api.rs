use serde::{Deserialize, Serialize};

use super::config::{clamp_speed, REST_HOST, XAI_PCM_SAMPLE_RATE};
use super::protocol::user_message_for_xai_error;

const VOICE_HTTP_TIMEOUT_SECS: u64 = 15;
const VOICE_PREVIEW_HTTP_TIMEOUT_SECS: u64 = 30;
const PREVIEW_SAMPLE_TEXT: &str = "Hello, this is a voice test.";

fn voice_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("build http client: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct XaiVoiceOption {
    pub voice_id: String,
    pub name: String,
    /// `builtIn` or `custom`
    #[serde(default = "default_voice_kind")]
    pub kind: String,
}

fn default_voice_kind() -> String {
    "builtIn".into()
}

#[derive(Debug, Deserialize)]
struct VoicesListResponse {
    voices: Vec<VoiceItem>,
}

#[derive(Debug, Deserialize)]
struct VoiceItem {
    voice_id: String,
    name: String,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomVoicesListResponse {
    #[serde(default)]
    voices: Vec<CustomVoiceItem>,
    #[serde(default)]
    custom_voices: Vec<CustomVoiceItem>,
}

#[derive(Debug, Deserialize)]
struct CustomVoiceItem {
    voice_id: String,
    name: String,
}

fn map_status_error(status: reqwest::StatusCode, body: &str) -> String {
    let token = format!("{} {body}", status.as_u16());
    user_message_for_xai_error(&token)
}

pub async fn test_api_key(api_key: &str) -> Result<(), String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/v1/tts/voices");
    let response = client
        .get(url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(map_status_error(status, &body))
}

pub async fn list_builtin_voices(api_key: &str) -> Result<Vec<XaiVoiceOption>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/v1/tts/voices");
    let response = client
        .get(url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(map_status_error(status, &body));
    }
    let page: VoicesListResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid xAI response: {e}"))?;
    Ok(page
        .voices
        .into_iter()
        .map(|item| XaiVoiceOption {
            voice_id: item.voice_id,
            name: if item.name.trim().is_empty() {
                item.language.unwrap_or_default()
            } else {
                item.name
            },
            kind: "builtIn".into(),
        })
        .collect())
}

async fn list_custom_voices(api_key: &str) -> Result<Vec<XaiVoiceOption>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/v1/custom-voices");
    let response = client
        .get(url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Ok(Vec::new());
    }
    if !response.status().is_success() {
        // Treat missing custom-voices access as empty, not fatal.
        return Ok(Vec::new());
    }
    let page: CustomVoicesListResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid xAI custom voices response: {e}"))?;
    let items = if page.voices.is_empty() {
        page.custom_voices
    } else {
        page.voices
    };
    Ok(items
        .into_iter()
        .map(|item| XaiVoiceOption {
            voice_id: item.voice_id,
            name: item.name,
            kind: "custom".into(),
        })
        .collect())
}

pub async fn list_voices(api_key: &str) -> Result<Vec<XaiVoiceOption>, String> {
    let mut voices = list_builtin_voices(api_key).await?;
    match list_custom_voices(api_key).await {
        Ok(custom) => voices.extend(custom),
        Err(_) => {}
    }
    Ok(voices)
}

pub async fn validate_voice(api_key: &str, voice_id: &str) -> Result<(), String> {
    let voice_id = voice_id.trim();
    if voice_id.is_empty() {
        return Err("xAI voice ID is required.".into());
    }
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let builtin_url = format!("{REST_HOST}/v1/tts/voices/{voice_id}");
    let response = client
        .get(&builtin_url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    if response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(map_status_error(status, &body));
    }
    let custom_url = format!("{REST_HOST}/v1/custom-voices/{voice_id}");
    let response = client
        .get(custom_url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(map_status_error(status, &body))
}

pub async fn preview_voice(
    api_key: &str,
    voice_id: &str,
    config: &crate::config::AppConfig,
    devices: &[crate::audio::AudioDeviceInfo],
) -> Result<(), String> {
    validate_voice(api_key, voice_id).await?;
    let client = voice_http_client(VOICE_PREVIEW_HTTP_TIMEOUT_SECS)?;
    let language = super::config::map_tts_language(&config.meeting_language);
    let speed = clamp_speed(config.xai.xai_speed);
    let body = serde_json::json!({
        "text": PREVIEW_SAMPLE_TEXT,
        "voice_id": voice_id,
        "language": language,
        "speed": speed,
        "output_format": {
            "codec": "pcm",
            "sample_rate": XAI_PCM_SAMPLE_RATE
        }
    });
    let response = client
        .post(format!("{REST_HOST}/v1/tts"))
        .bearer_auth(api_key.trim())
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Preview request failed: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(map_status_error(status, &err_body));
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
