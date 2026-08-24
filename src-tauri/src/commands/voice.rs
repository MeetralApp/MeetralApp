use serde::Deserialize;
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

use crate::config::AppConfig;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestElevenLabsApiKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListElevenLabsVoicesRequest {
    pub api_key: String,
}

#[tauri::command]
pub async fn list_elevenlabs_voices(
    request: ListElevenLabsVoicesRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::voice::ElevenLabsVoiceOption>, String> {
    let api_key = {
        let guard = store.lock().await;
        if request.api_key.trim().is_empty() {
            guard.elevenlabs.elevenlabs_api_key.clone()
        } else {
            request.api_key.clone()
        }
    };

    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is empty".into());
    }

    crate::voice::list_elevenlabs_voices(&api_key).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListElevenLabsModelsRequest {
    pub api_key: String,
}

#[tauri::command]
pub async fn list_elevenlabs_models(
    request: ListElevenLabsModelsRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::voice::ElevenLabsModelOption>, String> {
    let api_key = {
        let guard = store.lock().await;
        if request.api_key.trim().is_empty() {
            guard.elevenlabs.elevenlabs_api_key.clone()
        } else {
            request.api_key.clone()
        }
    };

    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is empty".into());
    }

    crate::voice::list_elevenlabs_models(&api_key).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSonioxVoicesRequest {
    pub api_key: String,
    #[serde(default)]
    pub preferred_model: Option<String>,
}

#[tauri::command]
pub async fn list_soniox_voices(
    request: ListSonioxVoicesRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::voice::SonioxVoiceOption>, String> {
    let (api_key, preferred) = {
        let guard = store.lock().await;
        let key = if request.api_key.trim().is_empty() {
            guard.soniox_api_key.clone()
        } else {
            request.api_key.clone()
        };
        let preferred = request
            .preferred_model
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                let m = guard.soniox.soniox_tts_model.trim();
                if m.is_empty() {
                    None
                } else {
                    Some(m.to_string())
                }
            });
        (key, preferred)
    };

    if api_key.trim().is_empty() {
        return Err("Soniox API key is empty".into());
    }

    crate::voice::list_soniox_voices_for_model(&api_key, preferred.as_deref()).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSonioxTtsModelsRequest {
    pub api_key: String,
}

#[tauri::command]
pub async fn list_soniox_tts_models(
    request: ListSonioxTtsModelsRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::voice::SonioxTtsModelOption>, String> {
    let api_key = {
        let guard = store.lock().await;
        if request.api_key.trim().is_empty() {
            guard.soniox_api_key.clone()
        } else {
            request.api_key.clone()
        }
    };

    if api_key.trim().is_empty() {
        return Err("Soniox API key is empty".into());
    }

    crate::voice::list_soniox_tts_models(&api_key).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSonioxSttModelsRequest {
    pub api_key: String,
}

#[tauri::command]
pub async fn list_soniox_stt_models(
    request: ListSonioxSttModelsRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::ai::LiveModelOption>, String> {
    let api_key = {
        let guard = store.lock().await;
        if request.api_key.trim().is_empty() {
            guard.soniox_api_key.clone()
        } else {
            request.api_key.clone()
        }
    };

    if api_key.trim().is_empty() {
        return Err("Soniox API key is empty".into());
    }

    crate::voice::list_soniox_stt_models(&api_key).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSonioxVoiceRequest {
    pub api_key: String,
    pub voice: String,
}

#[tauri::command]
pub async fn preview_soniox_voice(
    request: PreviewSonioxVoiceRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (config, devices) = {
        let guard = store.lock().await;
        (
            guard.clone(),
            crate::audio::list_devices_async().await.unwrap_or_default(),
        )
    };
    let api_key = if request.api_key.trim().is_empty() {
        config.soniox_api_key.clone()
    } else {
        request.api_key.clone()
    };
    let voice = if request.voice.trim().is_empty() {
        config.soniox.soniox_tts_voice.clone()
    } else {
        request.voice.clone()
    };
    if api_key.trim().is_empty() {
        return Err("Soniox API key is empty".into());
    }
    crate::voice::preview_soniox_voice(&api_key, &voice, &config, &devices).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateElevenLabsVoiceRequest {
    pub api_key: String,
    pub voice_id: String,
}

#[tauri::command]
pub async fn validate_elevenlabs_voice(
    request: ValidateElevenLabsVoiceRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (api_key, voice_id) = {
        let guard = store.lock().await;
        let api_key = if request.api_key.trim().is_empty() {
            guard.elevenlabs.elevenlabs_api_key.clone()
        } else {
            request.api_key.clone()
        };
        let voice_id = if request.voice_id.trim().is_empty() {
            guard.elevenlabs.elevenlabs_voice_id.clone()
        } else {
            request.voice_id.clone()
        };
        (api_key, voice_id)
    };
    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is empty".into());
    }
    crate::voice::validate_elevenlabs_voice(&api_key, &voice_id).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewElevenLabsVoiceRequest {
    pub api_key: String,
    pub voice_id: String,
}

#[tauri::command]
pub async fn preview_elevenlabs_voice(
    request: PreviewElevenLabsVoiceRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (config, devices) = {
        let guard = store.lock().await;
        (
            guard.clone(),
            crate::audio::list_devices_async().await.unwrap_or_default(),
        )
    };
    let api_key = if request.api_key.trim().is_empty() {
        config.elevenlabs.elevenlabs_api_key.clone()
    } else {
        request.api_key.clone()
    };
    let voice_id = if request.voice_id.trim().is_empty() {
        config.elevenlabs.elevenlabs_voice_id.clone()
    } else {
        request.voice_id.clone()
    };
    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is empty".into());
    }
    crate::voice::preview_elevenlabs_voice(&api_key, &voice_id, &config, &devices).await
}

#[tauri::command]
pub async fn test_elevenlabs_api_key(
    request: TestElevenLabsApiKeyRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let api_key = {
        let guard = store.lock().await;
        if request.api_key.trim().is_empty() {
            guard.elevenlabs.elevenlabs_api_key.clone()
        } else {
            request.api_key.clone()
        }
    };

    if api_key.trim().is_empty() {
        return Err("ElevenLabs API key is empty".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("build http client: {e}"))?;
    let response = client
        .get("https://api.elevenlabs.io/v1/user")
        .header("xi-api-key", api_key.clone())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if response.status().is_success() {
        let voices = client
            .get("https://api.elevenlabs.io/v2/voices?page_size=1")
            .header("xi-api-key", &api_key)
            .send()
            .await
            .map_err(|e| format!("Voices check failed: {e}"))?;
        if voices.status().is_success() {
            Ok(())
        } else {
            Err(
                "ElevenLabs API key works but cannot list voices — check Voices Read permission."
                    .into(),
            )
        }
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format!("ElevenLabs API error: {body}"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFishAudioApiKeyRequest {
    pub api_key: String,
}

fn resolve_fishaudio_api_key(stored: &str, request: &str) -> Result<String, String> {
    let api_key = if request.trim().is_empty() {
        stored.to_string()
    } else {
        request.to_string()
    };
    if api_key.trim().is_empty() {
        return Err("Fish Audio API key is empty".into());
    }
    Ok(api_key)
}

#[tauri::command]
pub async fn test_fishaudio_api_key(
    request: TestFishAudioApiKeyRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let api_key = {
        let guard = store.lock().await;
        resolve_fishaudio_api_key(&guard.fishaudio.fishaudio_api_key, &request.api_key)?
    };
    crate::providers::fishaudio::test_api_key(&api_key).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFishAudioVoicesRequest {
    pub api_key: String,
}

#[tauri::command]
pub async fn list_fishaudio_voices(
    request: ListFishAudioVoicesRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::providers::fishaudio::FishAudioVoiceOption>, String> {
    let api_key = {
        let guard = store.lock().await;
        resolve_fishaudio_api_key(&guard.fishaudio.fishaudio_api_key, &request.api_key)?
    };
    crate::providers::fishaudio::list_voices(&api_key).await
}

#[tauri::command]
pub fn list_fishaudio_models() -> Vec<crate::providers::fishaudio::FishAudioModelOption> {
    crate::providers::fishaudio::list_models()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateFishAudioVoiceRequest {
    pub api_key: String,
    pub voice_id: String,
}

#[tauri::command]
pub async fn validate_fishaudio_voice(
    request: ValidateFishAudioVoiceRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (api_key, voice_id) = {
        let guard = store.lock().await;
        let api_key =
            resolve_fishaudio_api_key(&guard.fishaudio.fishaudio_api_key, &request.api_key)?;
        let voice_id = if request.voice_id.trim().is_empty() {
            guard.fishaudio.fishaudio_voice_id.clone()
        } else {
            request.voice_id.clone()
        };
        (api_key, voice_id)
    };
    crate::providers::fishaudio::validate_voice(&api_key, &voice_id).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFishAudioVoiceRequest {
    pub api_key: String,
    pub voice_id: String,
}

#[tauri::command]
pub async fn preview_fishaudio_voice(
    request: PreviewFishAudioVoiceRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (config, devices) = {
        let guard = store.lock().await;
        (
            guard.clone(),
            crate::audio::list_devices_async().await.unwrap_or_default(),
        )
    };
    let api_key = resolve_fishaudio_api_key(&config.fishaudio.fishaudio_api_key, &request.api_key)?;
    let voice_id = if request.voice_id.trim().is_empty() {
        config.fishaudio.fishaudio_voice_id.clone()
    } else {
        request.voice_id.clone()
    };
    crate::providers::fishaudio::preview_voice(&api_key, &voice_id, &config, &devices).await
}
