use serde::Deserialize;
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

use crate::ai::{self, normalize_summary_model_for_provider, AiProvider, ProviderCatalog};
use crate::config::{AppConfig, ConfigView};
use crate::runtime::factories::{test_live_api_key, test_summary_api_key};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestApiKeyPurpose {
    #[default]
    Live,
    Summary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestApiKeyRequest {
    #[serde(default)]
    pub provider: AiProvider,
    pub api_key: String,
    #[serde(default)]
    pub purpose: TestApiKeyPurpose,
}

#[tauri::command]
pub async fn test_api_key(
    request: TestApiKeyRequest,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let (provider, api_key, setup_options, summary_model) = {
        let guard = store.lock().await;
        let provider = request.provider;
        let key = if request.api_key.trim().is_empty() {
            guard.api_key_for(provider).to_string()
        } else {
            request.api_key.clone()
        };
        (
            provider,
            key,
            guard.setup_options(),
            guard.summary_model.clone(),
        )
    };

    match request.purpose {
        TestApiKeyPurpose::Live => test_live_api_key(provider, &api_key, &setup_options).await,
        TestApiKeyPurpose::Summary => {
            let model = normalize_summary_model_for_provider(provider, &summary_model);
            test_summary_api_key(provider, &api_key, &model).await
        }
    }
}

#[tauri::command]
pub fn get_ai_catalog(provider: AiProvider) -> ProviderCatalog {
    ai::get_provider_catalog(provider)
}

/// Seed or return static live-model catalog (Gemini/OpenAI/Soniox fallback).
#[tauri::command]
pub fn seed_live_model_catalog(provider: AiProvider) -> Vec<ai::LiveModelOption> {
    ai::seed_live_models(provider)
}

#[tauri::command]
pub async fn migrate_config_for_provider(
    provider: AiProvider,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<ConfigView, String> {
    let mut guard = store.lock().await;
    guard.migrate_for_provider_switch(provider);
    Ok(ConfigView::from(&*guard))
}

#[tauri::command]
pub fn list_live_models(provider: AiProvider) -> Vec<String> {
    ai::get_provider_catalog(provider)
        .live_models
        .into_iter()
        .map(|model| model.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{TestApiKeyPurpose, TestApiKeyRequest};
    use crate::ai::AiProvider;

    #[test]
    fn test_api_key_purpose_defaults_to_live() {
        let request: TestApiKeyRequest = serde_json::from_value(serde_json::json!({
            "apiKey": "k",
            "provider": "gemini"
        }))
        .expect("deserialize");
        assert_eq!(request.purpose, TestApiKeyPurpose::Live);
        assert_eq!(request.provider, AiProvider::Gemini);
    }

    #[test]
    fn test_api_key_purpose_summary_parses() {
        let request: TestApiKeyRequest = serde_json::from_value(serde_json::json!({
            "apiKey": "k",
            "provider": "openAi",
            "purpose": "summary"
        }))
        .expect("deserialize");
        assert_eq!(request.purpose, TestApiKeyPurpose::Summary);
        assert_eq!(request.provider, AiProvider::OpenAi);
    }
}
