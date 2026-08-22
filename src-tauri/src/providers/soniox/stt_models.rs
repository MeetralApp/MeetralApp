//! Soniox STT model catalog — `GET /v1/models`.

use serde::Deserialize;

use crate::ai::types::{LanguageInfo, LiveModelOption};

const SONIOX_MODELS_URL: &str = "https://api.soniox.com/v1/models";
const VOICE_HTTP_TIMEOUT_SECS: u64 = 15;

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
    #[serde(default)]
    languages: Vec<ModelLanguage>,
}

#[derive(Debug, Deserialize)]
struct ModelLanguage {
    code: String,
    #[serde(default)]
    name: String,
}

/// List realtime STT models (`stt-rt-*`) with per-model languages.
pub async fn list_stt_models(api_key: &str) -> Result<Vec<LiveModelOption>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(VOICE_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("build http client: {e}"))?;
    let response = client
        .get(SONIOX_MODELS_URL)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .await
        .map_err(|e| format!("Soniox models request failed: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Soniox models read failed: {e}"))?;

    if !status.is_success() {
        return Err(format_soniox_http_error(status.as_u16(), &body));
    }

    let parsed: ModelsResponse =
        serde_json::from_str(&body).map_err(|e| format!("Soniox models parse failed: {e}"))?;

    Ok(live_models_from_response(&parsed.models))
}

fn live_models_from_response(models: &[ModelItem]) -> Vec<LiveModelOption> {
    models
        .iter()
        .filter(|m| m.id.starts_with("stt-rt-"))
        .map(|m| LiveModelOption {
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

fn format_soniox_http_error(status: u16, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("Soniox models HTTP {status}")
    } else {
        format!("Soniox models HTTP {status}: {trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_realtime_models_only() {
        let models = vec![
            ModelItem {
                id: "stt-rt-v5".to_string(),
                languages: vec![ModelLanguage {
                    code: "en".to_string(),
                    name: "English".to_string(),
                }],
            },
            ModelItem {
                id: "stt-async-v5".to_string(),
                languages: vec![ModelLanguage {
                    code: "vi".to_string(),
                    name: "Vietnamese".to_string(),
                }],
            },
        ];
        let live = live_models_from_response(&models);
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, "stt-rt-v5");
        assert_eq!(live[0].languages.len(), 1);
        assert_eq!(live[0].languages[0].code, "en");
        assert_eq!(live[0].languages[0].country_code, "US");
    }
}
