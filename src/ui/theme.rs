//! Shared typography and surfaces. DPI conversion remains in the Windows adapter.
use crate::core::{Appearance, parse_hex_color};
use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};

pub(super) const BODY_SIZE: f32 = 17.0;
pub(super) const DOCK_LABEL_SIZE: f32 = 16.0;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub field: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub hover: Color32,
}
impl Palette {
    pub fn for_dark(dark: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Self {
                background: rgb(15, 20, 29),
                panel: rgb(24, 32, 45),
                field: rgb(33, 44, 60),
                border: rgb(52, 66, 86),
                text: rgb(237, 242, 250),
                muted: rgb(155, 172, 196),
                accent: rgb(94, 157, 255),
                on_accent: rgb(8, 24, 48),
                hover: rgb(40, 64, 97),
            }
        } else {
            Self {
                background: rgb(242, 245, 249),
                panel: Color32::WHITE,
                field: rgb(235, 239, 245),
                border: rgb(218, 225, 234),
                text: rgb(27, 38, 58),
                muted: rgb(95, 112, 136),
                accent: rgb(45, 104, 210),
                on_accent: Color32::WHITE,
                hover: rgb(222, 234, 250),
            }
        }
    }
    pub fn of(ui: &egui::Ui) -> Self {
        let v = ui.visuals();
        Self {
            background: v.panel_fill,
            panel: v.window_fill,
            field: v.extreme_bg_color,
            border: v.widgets.inactive.bg_stroke.color,
            text: v.text_color(),
            muted: v.weak_text_color(),
            accent: v.selection.bg_fill,
            on_accent: v.selection.stroke.color,
            hover: v.widgets.hovered.bg_fill,
        }
    }
    pub fn resolve(dark: bool, appearance: &Appearance) -> Self {
        let mut p = Self::for_dark(dark);
        if appearance.palette == "custom" {
            let colors = appearance.colors.values().map(|value| {
                let [r, g, b] = parse_hex_color(value).unwrap_or([128, 128, 128]);
                Color32::from_rgb(r, g, b)
            });
            return Self {
                background: colors[0],
                panel: colors[1],
                field: colors[2],
                border: colors[3],
                text: colors[4],
                muted: colors[5],
                accent: colors[6],
                on_accent: colors[7],
                hover: colors[8],
            };
        }
        let (tint, accent) = match appearance.palette.as_str() {
            "ocean" => (
                Color32::from_rgb(20, 130, 175),
                if dark {
                    Color32::from_rgb(88, 210, 235)
                } else {
                    Color32::from_rgb(0, 100, 140)
                },
            ),
            "forest" => (
                Color32::from_rgb(40, 135, 80),
                if dark {
                    Color32::from_rgb(104, 218, 150)
                } else {
                    Color32::from_rgb(25, 115, 65)
                },
            ),
            "rose" => (
                Color32::from_rgb(165, 65, 120),
                if dark {
                    Color32::from_rgb(245, 150, 195)
                } else {
                    Color32::from_rgb(160, 45, 100)
                },
            ),
            _ => return p,
        };
        let mix = |base: Color32, weight: f32| {
            let a = base.to_array();
            let b = tint.to_array();
            Color32::from_rgb(
                (a[0] as f32 * (1.0 - weight) + b[0] as f32 * weight) as u8,
                (a[1] as f32 * (1.0 - weight) + b[1] as f32 * weight) as u8,
                (a[2] as f32 * (1.0 - weight) + b[2] as f32 * weight) as u8,
            )
        };
        p.background = mix(p.background, 0.08);
        p.panel = mix(p.panel, 0.10);
        p.field = mix(p.field, 0.15);
        p.border = mix(p.border, 0.25);
        p.hover = mix(p.hover, 0.25);
        p.accent = accent;
        p
    }
}

pub(super) fn configure(ctx: &egui::Context, appearance: &Appearance) {
    ctx.all_styles_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(BODY_SIZE));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(BODY_SIZE));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(25.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(15.0));
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 36.0;
        style.spacing.slider_width = 240.0;
        let p = Palette::resolve(style.visuals.dark_mode, appearance);
        let v = &mut style.visuals;
        v.panel_fill = p.background;
        v.window_fill = p.panel;
        v.faint_bg_color = p.field;
        v.extreme_bg_color = p.field;
        v.override_text_color = Some(p.text);
        v.weak_text_color = Some(p.muted);
        v.hyperlink_color = p.accent;
        v.selection.bg_fill = p.accent;
        v.selection.stroke = Stroke::new(1.0, p.on_accent);
        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.corner_radius = 8.into();
            widget.fg_stroke = Stroke::new(1.0, p.text);
            widget.bg_stroke = Stroke::new(1.0, p.border);
            widget.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_fill = p.panel;
        v.widgets.inactive.bg_fill = p.field;
        v.widgets.inactive.weak_bg_fill = p.field;
        v.widgets.hovered.bg_fill = p.hover;
        v.widgets.hovered.weak_bg_fill = p.hover;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
        v.widgets.active.bg_fill = p.hover;
        v.widgets.active.weak_bg_fill = p.hover;
        v.widgets.active.bg_stroke = Stroke::new(1.5, p.accent);
    });
}

pub(super) fn card<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let p = Palette::of(ui);
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().slider_width = (ui.available_width() - 250.0).clamp(100.0, 240.0);
            content(ui)
        })
        .inner
}
pub(super) fn subtitle(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(15.0)
            .color(Palette::of(ui).muted),
    );
}
pub(super) fn primary(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let p = Palette::of(ui);
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).color(p.on_accent))
            .fill(p.accent)
            .min_size(egui::vec2(120.0, 38.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_changes_reach_both_styles_and_custom_colors_reach_views() {
        let ctx = egui::Context::default();
        for palette in ["default", "ocean", "forest", "rose", "custom"] {
            let appearance = Appearance {
                palette: palette.into(),
                ..Default::default()
            };
            configure(&ctx, &appearance);
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let expected = Palette::resolve(theme == egui::Theme::Dark, &appearance);
                let style = ctx.style_of(theme);
                assert_eq!(style.visuals.selection.bg_fill, expected.accent);
                assert_eq!(style.visuals.window_fill, expected.panel);
                assert_eq!(style.visuals.weak_text_color(), expected.muted);
            }
        }
        let mut appearance = Appearance {
            palette: "custom".into(),
            ..Default::default()
        };
        appearance.colors.panel = "#123456".into();
        configure(&ctx, &appearance);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            assert_eq!(Palette::of(ui).panel, Color32::from_rgb(18, 52, 86));
        });
        output.textures_delta.clear();
        configure(&ctx, &Appearance::default());
        assert_ne!(
            ctx.style_of(egui::Theme::Dark).visuals.window_fill,
            Color32::from_rgb(18, 52, 86)
        );
    }
    #[test]
    fn both_themes_keep_readable_fonts_without_changing_zoom() {
        let ctx = egui::Context::default();
        configure(&ctx, &Appearance::default());
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let style = ctx.style_of(theme);
            assert_eq!(style.text_styles[&TextStyle::Body].size, 17.0);
            assert_eq!(style.text_styles[&TextStyle::Button].size, 17.0);
            assert!(style.spacing.interact_size.y >= 36.0);
        }
        assert_eq!(ctx.zoom_factor(), 1.0);
    }
}
