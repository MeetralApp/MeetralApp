use serde::{Deserialize, Serialize};

use crate::ai::types::LanguageInfo;

const SONIOX_TTS_MODELS_URL: &str = "https://api.soniox.com/v1/tts-models";
const VOICE_HTTP_TIMEOUT_SECS: u64 = 15;
const VOICE_PREVIEW_HTTP_TIMEOUT_SECS: u64 = 30;

fn voice_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("build http client: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxVoiceOption {
    pub id: String,
    pub name: String,
    pub gender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Persisted Soniox TTS model catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxTtsModelOption {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub languages: Vec<LanguageInfo>,
}

#[derive(Debug, Deserialize)]
struct TtsModelsResponse {
    models: Vec<TtsModelItem>,
}

#[derive(Debug, Deserialize)]
struct TtsModelItem {
    id: String,
    #[serde(default)]
    voices: Vec<TtsVoiceItem>,
    #[serde(default)]
    languages: Vec<TtsLanguageItem>,
}

#[derive(Debug, Deserialize)]
struct TtsVoiceItem {
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    gender: String,
}

#[derive(Debug, Deserialize)]
struct TtsLanguageItem {
    code: String,
    #[serde(default)]
    name: String,
}

async fn fetch_tts_models_response(api_key: &str) -> Result<TtsModelsResponse, String> {
    let client = voice_http_client(VOICE_HTTP_TIMEOUT_SECS)?;
    let response = client
        .get(SONIOX_TTS_MODELS_URL)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .await
        .map_err(|e| format!("Soniox TTS models request failed: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Soniox TTS models read failed: {e}"))?;

    if !status.is_success() {
        return Err(format_soniox_http_error(status.as_u16(), &body));
    }

    serde_json::from_str(&body).map_err(|e| format!("Soniox TTS models parse failed: {e}"))
}

/// List TTS models with per-model languages from `GET /v1/tts-models`.
pub async fn list_tts_models(api_key: &str) -> Result<Vec<SonioxTtsModelOption>, String> {
    let parsed = fetch_tts_models_response(api_key).await?;
    Ok(tts_models_from_response(&parsed.models))
}

/// List built-in Soniox TTS voices from `GET /v1/tts-models`.
///
/// Prefers voices on `preferred_model` (or the configured default); falls back to
/// the first model that includes voices.
pub async fn list_voices(
    api_key: &str,
    preferred_model: Option<&str>,
) -> Result<Vec<SonioxVoiceOption>, String> {
    let parsed = fetch_tts_models_response(api_key).await?;
    let preferred = preferred_model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(crate::providers::soniox::tts::config::DEFAULT_SONIOX_TTS_MODEL);
    Ok(voices_from_models(&parsed.models, preferred))
}

fn tts_models_from_response(models: &[TtsModelItem]) -> Vec<SonioxTtsModelOption> {
    models
        .iter()
        .filter(|m| !m.id.trim().is_empty())
        .map(|m| SonioxTtsModelOption {
            id: m.id.clone(),
            name: Some(m.id.clone()),
            languages: m
                .languages
                .iter()
                .map(|lang| {
                    LanguageInfo::new(
                        lang.code.clone(),
                        if lang.name.trim().is_empty() {
                            lang.code.clone()
                        } else {
                            lang.name.clone()
                        },
                    )
                })
                .collect(),
        })
        .collect()
}

const PREVIEW_SAMPLE_TEXT: &str = "Hello, this is a voice test.";
const SONIOX_TTS_GENERATE_URL: &str = "https://tts-rt.soniox.com/tts";

/// One-shot REST TTS preview played on the local playback device (same path as ElevenLabs).
pub async fn preview_voice(
    api_key: &str,
    voice: &str,
    config: &crate::config::AppConfig,
    devices: &[crate::audio::AudioDeviceInfo],
) -> Result<(), String> {
    let voice = crate::providers::soniox::tts::config::normalize_soniox_tts_voice(voice);
    let language = config.resolve_soniox_tts_language();
    let model = {
        let m = config.soniox.soniox_tts_inbound_model.trim();
        if m.is_empty() {
            let outbound = config.soniox.soniox_tts_outbound_model.trim();
            if outbound.is_empty() {
                crate::providers::soniox::tts::config::DEFAULT_SONIOX_TTS_MODEL
            } else {
                outbound
            }
        } else {
            m
        }
    };

    let client = voice_http_client(VOICE_PREVIEW_HTTP_TIMEOUT_SECS)?;
    let body = serde_json::json!({
        "text": PREVIEW_SAMPLE_TEXT,
        "voice": voice,
        "model": model,
        "language": language,
        "audio_format": "pcm_s16le",
        "sample_rate": crate::providers::soniox::tts::config::TTS_SAMPLE_RATE,
        "speed": crate::providers::soniox::tts::config::clamp_soniox_tts_speed(
            config.soniox.soniox_tts_inbound_speed,
        ),
    });
    let response = client
        .post(SONIOX_TTS_GENERATE_URL)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Soniox preview request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(format_soniox_http_error(status.as_u16(), &err_body));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Soniox preview read failed: {e}"))?;
    if bytes.len() < 2 || !bytes.len().is_multiple_of(2) {
        return Err("Soniox preview returned invalid audio".into());
    }
    let pcm: Vec<i16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| i16::from_le_bytes(*chunk))
        .collect();
    let pcm_48k = crate::audio::resampler::upsample_24k_to_48k(&pcm)
        .map_err(|e| format!("Soniox preview resample failed: {e}"))?;

    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
    let _ = tx.try_send(pcm_48k);
    drop(tx);

    let playback = crate::audio::start_playback_for_role(
        crate::audio::AudioRole::LocalPlayback,
        config,
        devices,
        rx,
    )
    .map_err(|e| format!("Soniox preview playback failed: {e}"))?;

    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    playback.stop();
    Ok(())
}

fn voices_from_models(models: &[TtsModelItem], preferred_model: &str) -> Vec<SonioxVoiceOption> {
    let preferred = models
        .iter()
        .find(|m| m.id == preferred_model && !m.voices.is_empty());
    let selected = preferred
        .or_else(|| models.iter().find(|m| !m.voices.is_empty()))
        .map(|m| m.voices.as_slice())
        .unwrap_or(&[]);

    let mut out: Vec<SonioxVoiceOption> = selected
        .iter()
        .filter(|v| !v.id.trim().is_empty())
        .map(|v| SonioxVoiceOption {
            id: v.id.clone(),
            name: v.id.clone(),
            gender: v.gender.clone(),
            description: non_empty_opt(&v.description),
        })
        .collect();

    out.sort_by_key(|a| a.name.to_ascii_lowercase());
    out.dedup_by(|a, b| a.id == b.id);
    out
}

fn non_empty_opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn format_soniox_http_error(status: u16, body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = value.get("message").and_then(|m| m.as_str()) {
            return format!("Soniox TTS models error ({status}): {msg}");
        }
    }
    let snippet: String = body.chars().take(160).collect();
    if snippet.is_empty() {
        format!("Soniox TTS models error ({status})")
    } else {
        format!("Soniox TTS models error ({status}): {snippet}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_preferred_model_voices() {
        let models = vec![
            TtsModelItem {
                id: "other".into(),
                voices: vec![TtsVoiceItem {
                    id: "Zed".into(),
                    description: "skip".into(),
                    gender: "male".into(),
                }],
                languages: vec![],
            },
            TtsModelItem {
                id: "tts-rt-v1".into(),
                voices: vec![
                    TtsVoiceItem {
                        id: "Noah".into(),
                        description: "Youthful".into(),
                        gender: "male".into(),
                    },
                    TtsVoiceItem {
                        id: "Adrian".into(),
                        description: "Deep".into(),
                        gender: "male".into(),
                    },
                    TtsVoiceItem {
                        id: "Maya".into(),
                        description: String::new(),
                        gender: "female".into(),
                    },
                ],
                languages: vec![TtsLanguageItem {
                    code: "en".into(),
                    name: "English".into(),
                }],
            },
        ];
        let voices = voices_from_models(&models, "tts-rt-v1");
        assert_eq!(voices.len(), 3);
        assert_eq!(voices[0].id, "Adrian");
        assert_eq!(voices[1].id, "Maya");
        assert_eq!(voices[2].id, "Noah");
        assert_eq!(voices[0].description.as_deref(), Some("Deep"));
        assert!(voices[1].description.is_none());

        let tts_models = tts_models_from_response(&models);
        assert_eq!(tts_models.len(), 2);
        assert_eq!(tts_models[1].languages[0].code, "en");
    }

    #[test]
    fn falls_back_to_first_model_with_voices() {
        let models = vec![TtsModelItem {
            id: "tts-rt-preview".into(),
            voices: vec![TtsVoiceItem {
                id: "Emma".into(),
                description: "Smooth".into(),
                gender: "female".into(),
            }],
            languages: vec![],
        }];
        let voices = voices_from_models(&models, "tts-rt-v1");
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].id, "Emma");
    }
}
