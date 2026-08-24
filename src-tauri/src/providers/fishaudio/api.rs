use serde::{Deserialize, Serialize};

use super::config::{normalize_tts_model, REST_HOST, TTS_MODEL_IDS};
use super::protocol::user_message_for_fishaudio_error;

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
pub struct FishAudioVoiceOption {
    pub voice_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FishAudioModelOption {
    pub model_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelListResponse {
    items: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    #[serde(rename = "_id")]
    id: String,
    title: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    state: String,
}

fn map_status_error(status: reqwest::StatusCode, body: &str) -> String {
    let token = format!("{} {body}", status.as_u16());
    user_message_for_fishaudio_error(&token)
}

pub fn list_models() -> Vec<FishAudioModelOption> {
    TTS_MODEL_IDS
        .iter()
        .map(|id| FishAudioModelOption {
            model_id: (*id).to_string(),
            name: match *id {
                "s2.1-pro" => "S2.1 Pro".into(),
                "s2.1-pro-free" => "S2.1 Pro (free, no latency SLA)".into(),
                "s2-pro" => "S2 Pro".into(),
                "s1" => "S1".into(),
                other => other.to_string(),
            },
            description: None,
        })
        .collect()
}

pub async fn test_api_key(api_key: &str) -> Result<(), String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/model?self=true&page_size=1");
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

pub async fn list_voices(api_key: &str) -> Result<Vec<FishAudioVoiceOption>, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/model?self=true&page_size=100");
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
    let page: ModelListResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid Fish Audio response: {e}"))?;
    Ok(page
        .items
        .into_iter()
        .filter(|item| item.r#type != "svc" && item.state != "failed")
        .map(|item| FishAudioVoiceOption {
            voice_id: item.id,
            name: item.title,
        })
        .collect())
}

pub async fn validate_voice(api_key: &str, voice_id: &str) -> Result<(), String> {
    let voice_id = voice_id.trim();
    if voice_id.is_empty() {
        return Err("Fish Audio voice ID is required.".into());
    }
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let url = format!("{REST_HOST}/model/{voice_id}");
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

pub async fn preview_voice(
    api_key: &str,
    voice_id: &str,
    config: &crate::config::AppConfig,
    devices: &[crate::audio::AudioDeviceInfo],
) -> Result<(), String> {
    validate_voice(api_key, voice_id).await?;
    let client = voice_http_client(VOICE_PREVIEW_HTTP_TIMEOUT_SECS)?;
    let model = normalize_tts_model(&config.fishaudio.fishaudio_tts_model);
    let body = serde_json::json!({
        "text": PREVIEW_SAMPLE_TEXT,
        "reference_id": voice_id,
        "format": "pcm",
        "sample_rate": super::config::FISH_PCM_SAMPLE_RATE,
        "latency": config.fishaudio.fishaudio_latency.as_api_str(),
        "temperature": config.fishaudio.fishaudio_temperature,
        "top_p": config.fishaudio.fishaudio_top_p,
        "prosody": {
            "speed": super::config::clamp_speed(config.fishaudio.fishaudio_speed),
            "volume": 0,
            "normalize_loudness": true
        }
    });
    let response = client
        .post(format!("{REST_HOST}/v1/tts"))
        .bearer_auth(api_key.trim())
        .header("Content-Type", "application/json")
        .header("model", model)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_models_include_s21_pro() {
        let models = list_models();
        assert!(models.iter().any(|m| m.model_id == "s2.1-pro"));
        assert!(models.iter().any(|m| m.model_id == "s2.1-pro-free"));
    }
}
