//! Prefer an installed regular JetBrains Mono, retaining egui's fallback fonts.
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::path::PathBuf;

pub(super) fn configure(ctx: &egui::Context) {
    let mut candidates = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let root = PathBuf::from(home).join(".local/share/fonts");
        for name in [
            "JetBrainsMono-Regular.ttf",
            "JetBrainsMonoNerdFontMono-Regular.ttf",
            "JetBrainsMonoNLNerdFontMono-Regular.ttf",
        ] {
            candidates.push(root.join(name));
        }
    }
    candidates.extend([
        PathBuf::from("/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf"),
        PathBuf::from("/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf"),
    ]);
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) {
        fonts.font_data.insert(
            "jetbrains-regular".into(),
            FontData::from_owned(bytes).into(),
        );
        for family in [FontFamily::Monospace, FontFamily::Proportional] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "jetbrains-regular".into());
        }
    }
    ctx.set_fonts(fonts);
    ctx.global_style_mut(|style| {
        for (kind, size) in [
            (TextStyle::Body, 14.0),
            (TextStyle::Button, 13.0),
            (TextStyle::Small, 12.0),
            (TextStyle::Monospace, 14.0),
            (TextStyle::Heading, 20.0),
        ] {
            style.text_styles.insert(kind, FontId::monospace(size));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regular_font_setup_preserves_fallbacks_and_renders_text() {
        let ctx = eframe::egui::Context::default();
        configure(&ctx);
        let mut output = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            ui.label("tb.dut.count[3:0] 0123456789 XZ →");
        });
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
        assert_eq!(
            ctx.global_style().text_styles[&eframe::egui::TextStyle::Body].size,
            14.0
        );
    }
}
