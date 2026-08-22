use serde::{Deserialize, Serialize};

use crate::voice::config::TtsSynthesisMode;

pub const ELEVENLABS_KEYCHAIN_ACCOUNT: &str = "elevenlabs_api_key";
pub const DEFAULT_TTS_MODEL: &str = "eleven_flash_v2_5";

/// Model IDs known to work with outbound clone WebSocket `stream-input`.
pub const CLONE_STREAM_INPUT_MODEL_IDS: &[&str] = &[
    "eleven_flash_v2_5",
    "eleven_flash_v2",
    "eleven_turbo_v2_5",
    "eleven_turbo_v2",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ElevenLabsChunkSchedulePreset {
    #[default]
    Live,
    Fast,
    Balanced,
    Quality,
}

pub fn supports_clone_stream_input(model_id: &str) -> bool {
    CLONE_STREAM_INPUT_MODEL_IDS.contains(&model_id)
}

pub const DEFAULT_OUTPUT_FORMAT: &str = "pcm_24000";
pub const ELEVENLABS_WS_HOST: &str = "wss://api.elevenlabs.io";

pub fn default_elevenlabs_tts_model() -> String {
    DEFAULT_TTS_MODEL.to_string()
}

pub fn default_elevenlabs_stability() -> f32 {
    0.5
}

pub fn default_elevenlabs_similarity_boost() -> f32 {
    0.75
}

pub fn default_elevenlabs_speed() -> f32 {
    1.0
}

pub fn default_elevenlabs_use_speaker_boost() -> bool {
    true
}

pub fn default_elevenlabs_chunk_schedule_preset() -> ElevenLabsChunkSchedulePreset {
    ElevenLabsChunkSchedulePreset::Live
}

pub fn default_elevenlabs_tts_language_auto() -> bool {
    true
}

pub fn default_elevenlabs_tts_synthesis_mode() -> TtsSynthesisMode {
    TtsSynthesisMode::Streaming
}

pub fn default_elevenlabs_playback_crossfade() -> bool {
    false
}

pub fn default_elevenlabs_crossfade_ms() -> u32 {
    8
}

/// ElevenLabs WS `voice_settings.speed` allows 0.7–1.2 inclusive. `f32` values such as
/// `1.2` become `1.200000047…` in JSON and can trigger `invalid_voice_settings`.
pub fn elevenlabs_speed_for_api(speed: f32) -> f64 {
    let v = (f64::from(speed.clamp(0.7, 1.2)) * 100.0).round() / 100.0;
    v.clamp(0.7, 1.2)
}

pub fn clamp_elevenlabs_speed(speed: f32) -> f32 {
    elevenlabs_speed_for_api(speed) as f32
}

pub fn chunk_schedule_for_preset(preset: ElevenLabsChunkSchedulePreset) -> [u32; 4] {
    match preset {
        ElevenLabsChunkSchedulePreset::Live => [50, 120, 200, 280],
        ElevenLabsChunkSchedulePreset::Fast => [50, 55, 60, 65],
        ElevenLabsChunkSchedulePreset::Balanced => [120, 160, 250, 290],
        ElevenLabsChunkSchedulePreset::Quality => [200, 280, 350, 400],
    }
}

/// Map deprecated UI presets to the supported Meeting / Smoother-start equivalents.
pub fn normalize_chunk_schedule_preset(
    preset: ElevenLabsChunkSchedulePreset,
) -> ElevenLabsChunkSchedulePreset {
    match preset {
        ElevenLabsChunkSchedulePreset::Fast => ElevenLabsChunkSchedulePreset::Live,
        ElevenLabsChunkSchedulePreset::Quality => ElevenLabsChunkSchedulePreset::Balanced,
        other => other,
    }
}

pub fn default_elevenlabs_auto_mode() -> bool {
    true
}

pub fn stream_input_url(
    voice_id: &str,
    model_id: &str,
    language_code: Option<&str>,
    auto_mode: bool,
) -> String {
    let auto = if auto_mode { "true" } else { "false" };
    let mut url = format!(
        "{}/v1/text-to-speech/{voice_id}/stream-input?model_id={model_id}&output_format={}&inactivity_timeout=60&auto_mode={auto}",
        ELEVENLABS_WS_HOST, DEFAULT_OUTPUT_FORMAT
    );
    if let Some(code) = language_code.map(str::trim).filter(|c| !c.is_empty()) {
        url.push_str(&format!("&language_code={code}"));
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_schedule_presets_match_documentation_defaults() {
        assert_eq!(
            chunk_schedule_for_preset(ElevenLabsChunkSchedulePreset::Live),
            [50, 120, 200, 280]
        );
        assert_eq!(
            chunk_schedule_for_preset(ElevenLabsChunkSchedulePreset::Fast),
            [50, 55, 60, 65]
        );
        assert_eq!(
            chunk_schedule_for_preset(ElevenLabsChunkSchedulePreset::Balanced),
            [120, 160, 250, 290]
        );
    }

    #[test]
    fn clamp_speed_to_ws_range() {
        assert!((clamp_elevenlabs_speed(0.5) - 0.7).abs() < f32::EPSILON);
        assert!((clamp_elevenlabs_speed(2.0) - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_for_api_rounds_f32_boundary() {
        assert!((elevenlabs_speed_for_api(1.2) - 1.2).abs() < f64::EPSILON);
        assert!(elevenlabs_speed_for_api(1.2) <= 1.2);
        assert!((elevenlabs_speed_for_api(0.7) - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_chunk_schedule_migrates_deprecated_presets() {
        assert_eq!(
            normalize_chunk_schedule_preset(ElevenLabsChunkSchedulePreset::Fast),
            ElevenLabsChunkSchedulePreset::Live
        );
        assert_eq!(
            normalize_chunk_schedule_preset(ElevenLabsChunkSchedulePreset::Quality),
            ElevenLabsChunkSchedulePreset::Balanced
        );
    }

    #[test]
    fn stream_input_url_appends_language_code() {
        let url = stream_input_url("voice", "eleven_flash_v2_5", Some("en"), true);
        assert!(url.contains("language_code=en"));
        assert!(url.contains("auto_mode=true"));
    }

    #[test]
    fn stream_input_url_can_disable_auto_mode() {
        let url = stream_input_url("voice", "eleven_flash_v2_5", None, false);
        assert!(url.contains("auto_mode=false"));
    }
}
