mod legacy;
mod soniox_migrate;
mod stored;

use anyhow::{Context, Result};
use tauri::AppHandle;

use crate::audio::{backfill_device_ids, list_devices};
use crate::config::AppConfig;

use legacy::parse_stored;
use stored::StoredConfig;

const CONFIG_FILE: &str = "config.json";

pub fn config_path(app: &AppHandle) -> Result<std::path::PathBuf> {
    let mut path = crate::app_data::ensure_dir(app)?;
    path.push(CONFIG_FILE);
    Ok(path)
}

pub fn backfill_and_save_if_needed(app: &AppHandle, config: &mut AppConfig) -> Result<bool> {
    let devices = match list_devices() {
        Ok(devices) => devices,
        Err(e) => {
            tracing::warn!("device backfill skipped: {e:#}");
            return Ok(false);
        }
    };

    if !backfill_device_ids(config, &devices) {
        return Ok(false);
    }

    tracing::info!("backfilled audio device IDs from configured names");
    save(app, config)?;
    Ok(true)
}

pub fn load(app: &AppHandle) -> Result<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let stored = parse_stored(&bytes)?;
    let migrate_legacy = stored
        .gemini_api_key
        .as_ref()
        .is_some_and(|key| !key.trim().is_empty())
        || stored
            .openai_api_key
            .as_ref()
            .is_some_and(|key| !key.trim().is_empty())
        || stored
            .elevenlabs_api_key
            .as_ref()
            .is_some_and(|key| !key.trim().is_empty())
        || serde_json::from_slice::<StoredConfig>(&bytes).is_err();
    let mut config = stored.into_app_config()?;

    if migrate_legacy {
        save(app, &config)?;
    }

    let _ = backfill_and_save_if_needed(app, &mut config);
    Ok(config)
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<()> {
    let path = config_path(app)?;
    let stored = StoredConfig::from_app_config(config)?;
    let bytes = serde_json::to_vec_pretty(&stored).context("serialize config")?;
    atomic_write(&path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Write `bytes` via a same-directory temp file then rename onto `path`.
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("write temp {}", tmp.display()))?;
    // On Windows, rename fails if the destination exists.
    if path.exists() {
        std::fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    }
    std::fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

pub fn load_merged(app: &AppHandle) -> Result<AppConfig> {
    load(app).or_else(|e| {
        tracing::warn!("config store load failed: {e:#}");
        Ok(AppConfig::default())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TranscriptLayout;

    #[test]
    fn stored_config_round_trips_transcript_layout() {
        let config = AppConfig {
            transcript_layout: TranscriptLayout::Stacked,
            ..AppConfig::default()
        };

        let stored = StoredConfig::from_app_config(&config).expect("serialize stored");
        let bytes = serde_json::to_vec(&stored).expect("json");
        assert!(String::from_utf8_lossy(&bytes).contains("stacked"));

        let parsed: StoredConfig = serde_json::from_slice(&bytes).expect("parse stored");
        let restored = parsed.into_app_config().expect("into app config");
        assert_eq!(restored.transcript_layout, TranscriptLayout::Stacked);
    }

    #[test]
    fn stored_config_defaults_transcript_layout_when_missing() {
        let json = br#"{"myLanguage":"vi","meetingLanguage":"en"}"#;
        let parsed: StoredConfig = serde_json::from_slice(json).expect("parse");
        let config = parsed.into_app_config().expect("into app config");
        assert_eq!(config.transcript_layout, TranscriptLayout::SideBySide);
        assert!(!config.auto_end_meeting);
        assert_eq!(
            config.auto_end_meeting_after_min,
            crate::config::DEFAULT_AUTO_END_MEETING_AFTER_MIN
        );
    }

    #[test]
    fn migrates_legacy_gemini_model_field() {
        let json = br#"{"geminiModel":"gemini-3.5-live-translate-preview","myLanguage":"vi","meetingLanguage":"en"}"#;
        let parsed: StoredConfig = serde_json::from_slice(json).expect("parse");
        assert_eq!(parsed.live_model, "gemini-3.5-live-translate-preview");
    }

    #[test]
    fn stored_config_ignores_legacy_ask_fields() {
        let json = br#"{
            "myLanguage":"vi",
            "meetingLanguage":"en",
            "askEnabled":true,
            "copilotEnabled":true,
            "embeddingModel":"nomic-embed-text"
        }"#;
        let parsed: StoredConfig = serde_json::from_slice(json).expect("parse");
        let config = parsed.into_app_config().expect("into app config");
        assert_eq!(config.my_language, "vi");
        assert_eq!(config.meeting_language, "en");
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir =
            std::env::temp_dir().join(format!("meetral-config-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join("config.json");
        atomic_write(&path, b"{\"a\":1}").expect("first write");
        atomic_write(&path, b"{\"a\":2}").expect("second write");
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(bytes, b"{\"a\":2}");
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
