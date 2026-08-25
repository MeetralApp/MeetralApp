pub const XAI_KEYCHAIN_ACCOUNT: &str = "xai_api_key";
pub const DEFAULT_VOICE_ID: &str = "eve";
pub const DEFAULT_SPEED: f32 = 1.0;
pub const XAI_PCM_SAMPLE_RATE: u32 = 24000;
pub const WS_HOST: &str = "wss://api.x.ai";
pub const REST_HOST: &str = "https://api.x.ai";

/// Hold this much PCM before the first mux emit of each utterance so later
/// `audio.delta` can queue before the DAC starts (removed in `ac931ad`, restored
/// as preroll — not OrderedPlayback / fade). 200 ms @ 24 kHz = 4800 samples.
pub const PLAYOUT_JITTER_MS: u32 = 200;
/// Same ~80 ms coalesce as other custom-voice workers (`EL_PCM_COALESCE_MIN_SAMPLES`).
pub const PCM_COALESCE_MIN_SAMPLES: usize = 1920;

pub fn playout_jitter_samples() -> usize {
    (XAI_PCM_SAMPLE_RATE as usize * PLAYOUT_JITTER_MS as usize) / 1000
}

/// Official BCP-47 codes from xAI TTS docs (plus `auto`). Canonical casing for wire.
pub const OFFICIAL_LANGUAGE_CODES: &[(&str, &str)] = &[
    ("auto", "auto"),
    ("en", "en"),
    ("ar-eg", "ar-EG"),
    ("ar-sa", "ar-SA"),
    ("ar-ae", "ar-AE"),
    ("bn", "bn"),
    ("zh", "zh"),
    ("fr", "fr"),
    ("de", "de"),
    ("hi", "hi"),
    ("id", "id"),
    ("it", "it"),
    ("ja", "ja"),
    ("ko", "ko"),
    ("pt-br", "pt-BR"),
    ("pt-pt", "pt-PT"),
    ("ru", "ru"),
    ("es-mx", "es-MX"),
    ("es-es", "es-ES"),
    ("tr", "tr"),
    ("vi", "vi"),
];

pub fn live_ws_url(
    language: &str,
    voice: &str,
    speed: f32,
    optimize_streaming_latency: u8,
) -> String {
    let language = urlencoding_simple(language);
    let voice = urlencoding_simple(voice);
    let speed = clamp_speed(speed);
    let latency = clamp_optimize_streaming_latency(optimize_streaming_latency);
    format!(
        "{WS_HOST}/v1/tts?language={language}&voice={voice}&codec=pcm&sample_rate={XAI_PCM_SAMPLE_RATE}&speed={speed}&optimize_streaming_latency={latency}"
    )
}

fn urlencoding_simple(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            _ => {
                for byte in ch.to_string().as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

pub fn clamp_speed(speed: f32) -> f32 {
    speed.clamp(0.7, 1.5)
}

/// Docs list 0/1/2; REST schema only documents 0|1. Handshake may 400 on 2 — callers clamp.
pub fn clamp_optimize_streaming_latency(level: u8) -> u8 {
    level.min(2)
}

/// Map Meetral language codes to xAI TTS `language` query values.
pub fn map_tts_language(code: &str) -> String {
    let trimmed = code.trim().to_ascii_lowercase().replace('_', "-");
    if trimmed.is_empty() {
        return "auto".into();
    }
    if let Some((_, canonical)) = OFFICIAL_LANGUAGE_CODES
        .iter()
        .find(|(key, _)| *key == trimmed.as_str())
    {
        return (*canonical).to_string();
    }
    match trimmed.as_str() {
        "ar" => "ar-SA".into(),
        "es" => "es-ES".into(),
        "pt" => "pt-PT".into(),
        _ => {
            let primary = trimmed.split('-').next().unwrap_or("");
            if let Some((_, canonical)) = OFFICIAL_LANGUAGE_CODES
                .iter()
                .find(|(key, _)| *key == primary)
            {
                (*canonical).to_string()
            } else {
                "auto".into()
            }
        }
    }
}

pub fn default_voice_id() -> String {
    DEFAULT_VOICE_ID.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_rate_matches_app_output() {
        assert_eq!(XAI_PCM_SAMPLE_RATE, crate::config::OUTPUT_SAMPLE_RATE);
    }

    #[test]
    fn speed_clamp_matches_xai_range() {
        assert_eq!(clamp_speed(0.5), 0.7);
        assert_eq!(clamp_speed(2.0), 1.5);
        assert_eq!(clamp_speed(1.0), 1.0);
    }

    #[test]
    fn language_mapper_table() {
        assert_eq!(map_tts_language("vi"), "vi");
        assert_eq!(map_tts_language("EN"), "en");
        assert_eq!(map_tts_language("ar"), "ar-SA");
        assert_eq!(map_tts_language("es"), "es-ES");
        assert_eq!(map_tts_language("pt"), "pt-PT");
        assert_eq!(map_tts_language("pt-BR"), "pt-BR");
        assert_eq!(map_tts_language("th"), "auto");
        assert_eq!(map_tts_language(""), "auto");
        assert_eq!(map_tts_language("zh-CN"), "zh");
    }

    #[test]
    fn latency_clamped_to_two() {
        assert_eq!(clamp_optimize_streaming_latency(0), 0);
        assert_eq!(clamp_optimize_streaming_latency(2), 2);
        assert_eq!(clamp_optimize_streaming_latency(9), 2);
    }

    #[test]
    fn live_ws_url_includes_pcm_24k() {
        let url = live_ws_url("en", "eve", 1.0, 1);
        assert!(url.contains("codec=pcm"));
        assert!(url.contains("sample_rate=24000"));
        assert!(url.contains("voice=eve"));
        assert!(url.contains("language=en"));
        assert!(url.contains("optimize_streaming_latency=1"));
    }

    #[test]
    fn pcm_coalesce_is_80ms_at_24k() {
        assert_eq!(PCM_COALESCE_MIN_SAMPLES, 1920);
    }

    #[test]
    fn playout_jitter_is_200ms_at_24k() {
        assert_eq!(playout_jitter_samples(), 4800);
    }
}
