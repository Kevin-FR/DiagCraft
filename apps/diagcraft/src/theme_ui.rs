//! Interface text roles, separate from document colours and muted decoration.
use eframe::egui::Color32;

pub(super) const INFO_TEXT: Color32 = Color32::from_rgb(82, 98, 113);
pub(super) const ACTION_TEXT: Color32 = Color32::from_rgb(0, 83, 180);
pub(super) const ACTION_FILL: Color32 = ACTION_TEXT;
pub(super) const WARNING_TEXT: Color32 = Color32::from_rgb(126, 78, 0);
pub(super) const ERROR_TEXT: Color32 = Color32::from_rgb(166, 37, 37);
pub(super) const HELP_BACKGROUND: Color32 = Color32::from_rgb(241, 244, 247);
pub(super) const CHANGE_INDICATOR: Color32 = Color32::from_rgb(202, 66, 66);

fn luminance(color: Color32) -> f64 {
    let linear = |channel: u8| {
        let v = f64::from(channel) / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

/// Empty-page prompts are UI, but the page beneath them is user-controlled.
/// Retain the informational role when legible; otherwise use opaque black/white.
pub(super) fn page_prompt_color(background: Color32) -> Color32 {
    let light = luminance(background);
    let info = luminance(INFO_TEXT);
    let ratio = (light.max(info) + 0.05) / (light.min(info) + 0.05);
    if ratio >= 4.5 {
        INFO_TEXT
    } else if (light + 0.05) / 0.05 >= 1.05 / (light + 0.05) {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}
