pub const FISHAUDIO_KEYCHAIN_ACCOUNT: &str = "fishaudio_api_key";
pub const DEFAULT_TTS_MODEL: &str = "s2.1-pro";
pub const DEFAULT_TEMPERATURE: f32 = 0.7;
pub const DEFAULT_SPEED: f32 = 1.0;
pub const DEFAULT_TOP_P: f32 = 0.7;
pub const FISH_PCM_SAMPLE_RATE: u32 = 24000;
pub const WS_HOST: &str = "wss://api.fish.audio";
pub const REST_HOST: &str = "https://api.fish.audio";

pub const TTS_MODEL_IDS: &[&str] = &["s2.1-pro", "s2.1-pro-free", "s2-pro", "s1"];

pub fn normalize_tts_model(model_id: &str) -> &'static str {
    let trimmed = model_id.trim();
    TTS_MODEL_IDS
        .iter()
        .copied()
        .find(|id| *id == trimmed)
        .unwrap_or(DEFAULT_TTS_MODEL)
}

pub fn live_ws_url() -> String {
    format!("{WS_HOST}/v1/tts/live")
}

pub fn clamp_speed(speed: f32) -> f32 {
    speed.clamp(0.5, 2.0)
}

pub fn clamp_temperature(temperature: f32) -> f32 {
    temperature.clamp(0.0, 1.0)
}

pub fn clamp_top_p(top_p: f32) -> f32 {
    top_p.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_rate_matches_app_output() {
        assert_eq!(FISH_PCM_SAMPLE_RATE, crate::config::OUTPUT_SAMPLE_RATE);
    }

    #[test]
    fn unknown_model_falls_back_to_s21_pro() {
        assert_eq!(normalize_tts_model("nope"), "s2.1-pro");
        assert_eq!(normalize_tts_model("s2.1-pro-free"), "s2.1-pro-free");
    }
}
