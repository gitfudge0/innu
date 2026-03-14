use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Stroke, Style,
    TextStyle, Visuals,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Compact,
    Narrow,
    Standard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeChoice {
    pub base: BaseMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePrefs {
    pub base: BaseMode,
}

#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub colors: ColorTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
    pub strokes: StrokeTokens,
}

#[derive(Debug, Clone)]
pub struct ColorTokens {
    pub window_bg: Color32,
    pub panel_bg: Color32,
    pub elevated_panel_bg: Color32,
    pub border: Color32,
    pub muted_border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub disabled_text: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub input_bg: Color32,
    pub input_border: Color32,
    pub hover_bg: Color32,
    pub pressed_bg: Color32,
    pub focus_border: Color32,
    pub selected_fill: Color32,
}

#[derive(Debug, Clone)]
pub struct SpacingTokens {
    pub micro: f32,
    pub tight: f32,
    pub control: f32,
    pub standard: f32,
    pub section: f32,
    pub major: f32,
    pub text_stack_gap: f32,
    pub inline_cluster_gap: f32,
    pub control_to_label_gap: f32,
    pub status_to_section_gap: f32,
    pub section_body_gap: f32,
    pub row_internal_gap: f32,
    pub row_to_row_gap: f32,
    pub row_height: f32,
    pub button_height: f32,
    pub page_padding: f32,
    pub max_content_width: f32,
}

#[derive(Debug, Clone)]
pub struct TypographyTokens {
    pub title: f32,
    pub section: f32,
    pub body: f32,
    pub helper: f32,
    pub micro: f32,
}

#[derive(Debug, Clone)]
pub struct StrokeTokens {
    pub standard: f32,
    pub focus: f32,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self {
            base: BaseMode::Dark,
        }
    }
}

impl ThemeChoice {
    pub fn toggle_base(&mut self) {
        self.base = match self.base {
            BaseMode::Light => BaseMode::Dark,
            BaseMode::Dark => BaseMode::Light,
        };
    }

    pub fn tokens(self, layout: LayoutClass) -> ThemeTokens {
        let colors = match self.base {
            BaseMode::Light => ColorTokens {
                window_bg: rgb(0xF5F3EE),
                panel_bg: rgb(0xFFFDF8),
                elevated_panel_bg: rgb(0xF1EEE7),
                border: rgb(0x1F1A17),
                muted_border: rgb(0x8E877F),
                text_primary: rgb(0x171311),
                text_secondary: rgb(0x5E5750),
                disabled_text: rgb(0x8A837C),
                accent: rgb(0xB56A1E),
                accent_text: rgb(0xFFF8F0),
                success: rgb(0x2F6B45),
                warning: rgb(0x8A5A12),
                error: rgb(0x8C2F2F),
                input_bg: rgb(0xFFFDF8),
                input_border: rgb(0x1F1A17),
                hover_bg: rgb(0xEFEAE2),
                pressed_bg: rgb(0xE4DED6),
                focus_border: rgb(0xB56A1E),
                selected_fill: rgb(0xF3E1CF),
            },
            BaseMode::Dark => ColorTokens {
                window_bg: rgb(0x101010),
                panel_bg: rgb(0x151515),
                elevated_panel_bg: rgb(0x1A1A1A),
                border: rgb(0xD6D0C7),
                muted_border: rgb(0x6B675F),
                text_primary: rgb(0xF2EEE7),
                text_secondary: rgb(0xB1AAA1),
                disabled_text: rgb(0x7A746B),
                accent: rgb(0xD0893C),
                accent_text: rgb(0x15110D),
                success: rgb(0x6FA57E),
                warning: rgb(0xD2A45B),
                error: rgb(0xD07C7C),
                input_bg: rgb(0x151515),
                input_border: rgb(0xD6D0C7),
                hover_bg: rgb(0x1C1C1C),
                pressed_bg: rgb(0x232323),
                focus_border: rgb(0xD0893C),
                selected_fill: rgb(0x261C12),
            },
        };

        ThemeTokens {
            colors,
            spacing: spacing_tokens(layout),
            typography: typography_tokens(layout),
            strokes: StrokeTokens {
                standard: 1.0,
                focus: 1.0,
            },
        }
    }
}

pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "ibm_plex_mono".into(),
        FontData::from_static(include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "material_symbols_sharp".into(),
        FontData::from_static(include_bytes!(
            "../../assets/fonts/MaterialSymbolsSharp.ttf"
        ))
        .into(),
    );

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "ibm_plex_mono".into());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "ibm_plex_mono".into());
    fonts
        .families
        .entry(FontFamily::Name("material_symbols_sharp".into()))
        .or_default()
        .insert(0, "material_symbols_sharp".into());

    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &Context, theme: ThemeChoice, layout: LayoutClass) {
    install_fonts(ctx);

    let tokens = theme.tokens(layout);
    let mut style: Style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(tokens.spacing.tight, tokens.spacing.tight);
    style.spacing.button_padding = egui::vec2(tokens.spacing.control, tokens.spacing.tight);

    style.text_styles = BTreeMap::from([
        (
            TextStyle::Heading,
            FontId::new(tokens.typography.title, FontFamily::Monospace),
        ),
        (
            TextStyle::Name("section".into()),
            FontId::new(tokens.typography.section, FontFamily::Monospace),
        ),
        (
            TextStyle::Body,
            FontId::new(tokens.typography.body, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(tokens.typography.body, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(tokens.typography.helper, FontFamily::Monospace),
        ),
        (
            TextStyle::Monospace,
            FontId::new(tokens.typography.body, FontFamily::Monospace),
        ),
    ]);

    let mut visuals = match theme.base {
        BaseMode::Light => Visuals::light(),
        BaseMode::Dark => Visuals::dark(),
    };
    visuals.override_text_color = Some(tokens.colors.text_primary);
    visuals.panel_fill = tokens.colors.window_bg;
    visuals.window_fill = tokens.colors.panel_bg;
    visuals.window_stroke = Stroke::new(tokens.strokes.standard, tokens.colors.border);
    visuals.window_corner_radius = 0.into();
    visuals.selection.bg_fill = tokens.colors.selected_fill;
    visuals.selection.stroke = Stroke::new(tokens.strokes.standard, tokens.colors.focus_border);
    visuals.widgets.noninteractive.bg_fill = tokens.colors.window_bg;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.colors.text_primary);
    visuals.widgets.inactive.bg_fill = tokens.colors.input_bg;
    visuals.widgets.inactive.weak_bg_fill = tokens.colors.panel_bg;
    visuals.widgets.inactive.bg_stroke =
        Stroke::new(tokens.strokes.standard, tokens.colors.input_border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.colors.text_primary);
    visuals.widgets.hovered.bg_fill = tokens.colors.hover_bg;
    visuals.widgets.hovered.weak_bg_fill = tokens.colors.hover_bg;
    visuals.widgets.hovered.bg_stroke =
        Stroke::new(tokens.strokes.standard, tokens.colors.focus_border);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.colors.text_primary);
    visuals.widgets.active.bg_fill = tokens.colors.pressed_bg;
    visuals.widgets.active.weak_bg_fill = tokens.colors.pressed_bg;
    visuals.widgets.active.bg_stroke =
        Stroke::new(tokens.strokes.focus, tokens.colors.focus_border);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, tokens.colors.text_primary);
    visuals.extreme_bg_color = tokens.colors.window_bg;
    visuals.faint_bg_color = tokens.colors.elevated_panel_bg;
    visuals.code_bg_color = tokens.colors.elevated_panel_bg;
    visuals.text_cursor.stroke = Stroke::new(tokens.strokes.standard, tokens.colors.focus_border);
    style.visuals = visuals;

    ctx.set_style(style);
}

pub fn layout_class_for_width(width: f32) -> LayoutClass {
    if width < 360.0 {
        LayoutClass::Compact
    } else if width < 520.0 {
        LayoutClass::Narrow
    } else {
        LayoutClass::Standard
    }
}

pub fn load_theme_prefs() -> ThemeChoice {
    let Ok(path) = prefs_path() else {
        return ThemeChoice::default();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return ThemeChoice::default();
    };
    toml::from_str::<ThemePrefs>(&raw)
        .map(|prefs| ThemeChoice { base: prefs.base })
        .unwrap_or_default()
}

pub fn save_theme_prefs(theme: ThemeChoice) -> anyhow::Result<()> {
    let path = prefs_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let prefs = ThemePrefs { base: theme.base };
    fs::write(path, toml::to_string_pretty(&prefs)?)?;
    Ok(())
}

fn prefs_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "gitfudge", "innu")
        .ok_or_else(|| anyhow::anyhow!("failed to resolve XDG configuration directory"))?;
    Ok(project_dirs.config_dir().join("theme.toml"))
}

fn rgb(value: u32) -> Color32 {
    Color32::from_rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}

fn spacing_tokens(layout: LayoutClass) -> SpacingTokens {
    match layout {
        LayoutClass::Standard => SpacingTokens {
            micro: 4.0,
            tight: 8.0,
            control: 12.0,
            standard: 16.0,
            section: 24.0,
            major: 32.0,
            text_stack_gap: 2.0,
            inline_cluster_gap: 8.0,
            control_to_label_gap: 8.0,
            status_to_section_gap: 20.0,
            section_body_gap: 12.0,
            row_internal_gap: 4.0,
            row_to_row_gap: 12.0,
            row_height: 56.0,
            button_height: 32.0,
            page_padding: 24.0,
            max_content_width: 960.0,
        },
        LayoutClass::Narrow => SpacingTokens {
            micro: 4.0,
            tight: 6.0,
            control: 10.0,
            standard: 12.0,
            section: 16.0,
            major: 24.0,
            text_stack_gap: 2.0,
            inline_cluster_gap: 6.0,
            control_to_label_gap: 6.0,
            status_to_section_gap: 14.0,
            section_body_gap: 10.0,
            row_internal_gap: 3.0,
            row_to_row_gap: 10.0,
            row_height: 48.0,
            button_height: 30.0,
            page_padding: 16.0,
            max_content_width: 960.0,
        },
        LayoutClass::Compact => SpacingTokens {
            micro: 2.0,
            tight: 4.0,
            control: 8.0,
            standard: 10.0,
            section: 12.0,
            major: 16.0,
            text_stack_gap: 2.0,
            inline_cluster_gap: 4.0,
            control_to_label_gap: 4.0,
            status_to_section_gap: 10.0,
            section_body_gap: 8.0,
            row_internal_gap: 2.0,
            row_to_row_gap: 8.0,
            row_height: 44.0,
            button_height: 28.0,
            page_padding: 12.0,
            max_content_width: 960.0,
        },
    }
}

fn typography_tokens(layout: LayoutClass) -> TypographyTokens {
    match layout {
        LayoutClass::Standard => TypographyTokens {
            title: 28.0,
            section: 18.0,
            body: 14.0,
            helper: 12.0,
            micro: 11.0,
        },
        LayoutClass::Narrow => TypographyTokens {
            title: 24.0,
            section: 16.0,
            body: 13.0,
            helper: 11.0,
            micro: 10.0,
        },
        LayoutClass::Compact => TypographyTokens {
            title: 20.0,
            section: 14.0,
            body: 12.0,
            helper: 10.0,
            micro: 9.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_layout_classes_from_width() {
        assert_eq!(layout_class_for_width(300.0), LayoutClass::Compact);
        assert_eq!(layout_class_for_width(400.0), LayoutClass::Narrow);
        assert_eq!(layout_class_for_width(700.0), LayoutClass::Standard);
    }
}
