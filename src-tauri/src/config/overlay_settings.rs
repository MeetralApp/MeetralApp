use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPosition {
    #[default]
    BottomCenter,
    BottomLeft,
    BottomRight,
    TopRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub position: OverlayPosition,
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
    #[serde(default = "default_true")]
    pub show_inbound: bool,
    #[serde(default = "default_true")]
    pub show_outbound: bool,
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,
    #[serde(default)]
    pub click_through: bool,
    #[serde(default = "default_true")]
    pub hide_from_capture: bool,
    #[serde(default = "default_true")]
    pub auto_show_with_session: bool,
    #[serde(default = "default_overlay_width")]
    pub width: f64,
    #[serde(default = "default_overlay_height")]
    pub height: f64,
}

fn default_true() -> bool {
    true
}

fn default_opacity() -> f32 {
    0.92
}

fn default_font_scale() -> f32 {
    1.0
}

pub fn default_overlay_width() -> f64 {
    420.0
}

pub fn default_overlay_height() -> f64 {
    280.0
}

pub const OVERLAY_MIN_WIDTH: f64 = 320.0;
pub const OVERLAY_MIN_HEIGHT: f64 = 200.0;
pub const OVERLAY_MAX_WIDTH: f64 = 900.0;
pub const OVERLAY_MAX_HEIGHT: f64 = 700.0;

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: default_opacity(),
            position: OverlayPosition::BottomCenter,
            offset_x: 0.0,
            offset_y: 0.0,
            show_inbound: true,
            show_outbound: true,
            font_scale: default_font_scale(),
            click_through: false,
            hide_from_capture: true,
            auto_show_with_session: true,
            width: default_overlay_width(),
            height: default_overlay_height(),
        }
    }
}

impl OverlaySettings {
    pub fn normalize(&mut self) {
        self.opacity = self.opacity.clamp(0.45, 1.0);
        self.font_scale = self.font_scale.clamp(0.85, 1.35);
        self.width = self.width.clamp(OVERLAY_MIN_WIDTH, OVERLAY_MAX_WIDTH);
        self.height = self.height.clamp(OVERLAY_MIN_HEIGHT, OVERLAY_MAX_HEIGHT);
        if !self.show_inbound && !self.show_outbound {
            self.show_inbound = true;
            self.show_outbound = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_forces_at_least_one_direction() {
        let mut s = OverlaySettings {
            show_inbound: false,
            show_outbound: false,
            ..Default::default()
        };
        s.normalize();
        assert!(s.show_inbound && s.show_outbound);
    }

    #[test]
    fn normalize_clamps_opacity_and_font() {
        let mut s = OverlaySettings {
            opacity: 0.1,
            font_scale: 3.0,
            ..Default::default()
        };
        s.normalize();
        assert!((s.opacity - 0.45).abs() < f32::EPSILON);
        assert!((s.font_scale - 1.35).abs() < f32::EPSILON);
    }

    #[test]
    fn normalize_clamps_width_and_height() {
        let mut s = OverlaySettings {
            width: 100.0,
            height: 50.0,
            ..Default::default()
        };
        s.normalize();
        assert!((s.width - OVERLAY_MIN_WIDTH).abs() < f64::EPSILON);
        assert!((s.height - OVERLAY_MIN_HEIGHT).abs() < f64::EPSILON);

        s.width = 2000.0;
        s.height = 2000.0;
        s.normalize();
        assert!((s.width - OVERLAY_MAX_WIDTH).abs() < f64::EPSILON);
        assert!((s.height - OVERLAY_MAX_HEIGHT).abs() < f64::EPSILON);
    }
}
