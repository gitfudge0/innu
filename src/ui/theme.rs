use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Stroke, Style,
    TextStyle, Visuals,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

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

#[derive(Debug, Clone, Default)]
pub struct AppearanceOverrides {
    light: SemanticPaletteOverride,
    dark: SemanticPaletteOverride,
    ui_font: Option<LoadedFont>,
}

#[derive(Debug, Clone)]
struct LoadedFont {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AppearancePrefs {
    #[serde(default)]
    fonts: FontOverrides,
    #[serde(default)]
    light: ModeAppearancePrefs,
    #[serde(default)]
    dark: ModeAppearancePrefs,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct FontOverrides {
    ui: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ModeAppearancePrefs {
    #[serde(default)]
    colors: ColorOverrideInputs,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ColorOverrideInputs {
    background: Option<String>,
    surface: Option<String>,
    border: Option<String>,
    text: Option<String>,
    text_muted: Option<String>,
    accent: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SemanticPaletteOverride {
    background: Option<Color32>,
    surface: Option<Color32>,
    border: Option<Color32>,
    text: Option<Color32>,
    text_muted: Option<Color32>,
    accent: Option<Color32>,
    success: Option<Color32>,
    warning: Option<Color32>,
    error: Option<Color32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemanticPalette {
    background: Color32,
    surface: Color32,
    border: Color32,
    text: Color32,
    text_muted: Color32,
    accent: Color32,
    success: Color32,
    warning: Color32,
    error: Color32,
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

    pub fn tokens(self, layout: LayoutClass, appearance: &AppearanceOverrides) -> ThemeTokens {
        let palette = appearance
            .palette_for(self.base)
            .apply(base_semantic_palette(self.base));
        let colors = derive_color_tokens(self.base, palette);

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

impl AppearanceOverrides {
    fn palette_for(&self, mode: BaseMode) -> &SemanticPaletteOverride {
        match mode {
            BaseMode::Light => &self.light,
            BaseMode::Dark => &self.dark,
        }
    }
}

impl SemanticPaletteOverride {
    fn apply(&self, base: SemanticPalette) -> SemanticPalette {
        SemanticPalette {
            background: self.background.unwrap_or(base.background),
            surface: self.surface.unwrap_or(base.surface),
            border: self.border.unwrap_or(base.border),
            text: self.text.unwrap_or(base.text),
            text_muted: self.text_muted.unwrap_or(base.text_muted),
            accent: self.accent.unwrap_or(base.accent),
            success: self.success.unwrap_or(base.success),
            warning: self.warning.unwrap_or(base.warning),
            error: self.error.unwrap_or(base.error),
        }
    }
}

pub fn load_appearance_overrides() -> AppearanceOverrides {
    let Ok(path) = appearance_path() else {
        return AppearanceOverrides::default();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return AppearanceOverrides::default();
    };

    match parse_appearance_overrides(&raw) {
        Ok(overrides) => overrides,
        Err(error) => {
            warn!(
                path = %path.display(),
                "failed to parse appearance overrides: {error}"
            );
            AppearanceOverrides::default()
        }
    }
}

pub fn install_fonts(ctx: &Context, appearance: &AppearanceOverrides) {
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

    let primary_font = if let Some(font) = &appearance.ui_font {
        fonts.font_data.insert(
            font.name.clone().into(),
            FontData::from_owned(font.bytes.clone()).into(),
        );
        font.name.as_str()
    } else {
        "ibm_plex_mono"
    };

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, primary_font.into());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, primary_font.into());
    fonts
        .families
        .entry(FontFamily::Name("material_symbols_sharp".into()))
        .or_default()
        .insert(0, "material_symbols_sharp".into());

    ctx.set_fonts(fonts);
}

pub fn apply_theme(
    ctx: &Context,
    theme: ThemeChoice,
    layout: LayoutClass,
    appearance: &AppearanceOverrides,
) {
    let tokens = theme.tokens(layout, appearance);
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

fn appearance_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "gitfudge", "innu")
        .ok_or_else(|| anyhow::anyhow!("failed to resolve XDG configuration directory"))?;
    Ok(project_dirs.config_dir().join("appearance.toml"))
}

fn parse_appearance_overrides(raw: &str) -> anyhow::Result<AppearanceOverrides> {
    let prefs = toml::from_str::<AppearancePrefs>(raw)?;
    Ok(AppearanceOverrides {
        light: parse_color_overrides(&prefs.light.colors, "light.colors"),
        dark: parse_color_overrides(&prefs.dark.colors, "dark.colors"),
        ui_font: prefs.fonts.ui.and_then(|path| load_font_override(&path)),
    })
}

fn parse_color_overrides(input: &ColorOverrideInputs, scope: &str) -> SemanticPaletteOverride {
    SemanticPaletteOverride {
        background: parse_optional_color(
            input.background.as_deref(),
            &format!("{scope}.background"),
        ),
        surface: parse_optional_color(input.surface.as_deref(), &format!("{scope}.surface")),
        border: parse_optional_color(input.border.as_deref(), &format!("{scope}.border")),
        text: parse_optional_color(input.text.as_deref(), &format!("{scope}.text")),
        text_muted: parse_optional_color(
            input.text_muted.as_deref(),
            &format!("{scope}.text_muted"),
        ),
        accent: parse_optional_color(input.accent.as_deref(), &format!("{scope}.accent")),
        success: parse_optional_color(input.success.as_deref(), &format!("{scope}.success")),
        warning: parse_optional_color(input.warning.as_deref(), &format!("{scope}.warning")),
        error: parse_optional_color(input.error.as_deref(), &format!("{scope}.error")),
    }
}

fn parse_optional_color(raw: Option<&str>, field_name: &str) -> Option<Color32> {
    let raw = raw?;
    match parse_hex_color(raw) {
        Ok(color) => Some(color),
        Err(error) => {
            warn!("ignoring invalid appearance color for {field_name}: {error}");
            None
        }
    }
}

fn parse_hex_color(raw: &str) -> anyhow::Result<Color32> {
    let value = raw.trim().trim_start_matches('#');
    anyhow::ensure!(
        value.len() == 6,
        "expected a 6-digit hex color, got `{raw}`"
    );
    let color = u32::from_str_radix(value, 16)?;
    Ok(rgb(color))
}

fn load_font_override(path: &Path) -> Option<LoadedFont> {
    match fs::read(path) {
        Ok(bytes) => Some(LoadedFont {
            name: format!(
                "user_ui_font_{}",
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("custom")
            ),
            bytes,
        }),
        Err(error) => {
            warn!(
                path = %path.display(),
                "ignoring unreadable appearance font override: {error}"
            );
            None
        }
    }
}

fn base_semantic_palette(mode: BaseMode) -> SemanticPalette {
    match mode {
        BaseMode::Light => SemanticPalette {
            background: rgb(0xF5F3EE),
            surface: rgb(0xFFFDF8),
            border: rgb(0x1F1A17),
            text: rgb(0x171311),
            text_muted: rgb(0x5E5750),
            accent: rgb(0xB56A1E),
            success: rgb(0x2F6B45),
            warning: rgb(0x8A5A12),
            error: rgb(0x8C2F2F),
        },
        BaseMode::Dark => SemanticPalette {
            background: rgb(0x101010),
            surface: rgb(0x151515),
            border: rgb(0xD6D0C7),
            text: rgb(0xF2EEE7),
            text_muted: rgb(0xB1AAA1),
            accent: rgb(0xD0893C),
            success: rgb(0x6FA57E),
            warning: rgb(0xD2A45B),
            error: rgb(0xD07C7C),
        },
    }
}

fn derive_color_tokens(mode: BaseMode, palette: SemanticPalette) -> ColorTokens {
    let emphasis = match mode {
        BaseMode::Light => 0.08,
        BaseMode::Dark => 0.06,
    };
    let pressed = match mode {
        BaseMode::Light => 0.14,
        BaseMode::Dark => 0.12,
    };
    let selected = match mode {
        BaseMode::Light => 0.22,
        BaseMode::Dark => 0.3,
    };

    ColorTokens {
        window_bg: palette.background,
        panel_bg: palette.surface,
        elevated_panel_bg: blend(palette.surface, palette.text, 0.04),
        border: palette.border,
        muted_border: blend(palette.border, palette.background, 0.45),
        text_primary: palette.text,
        text_secondary: palette.text_muted,
        disabled_text: blend(palette.text_muted, palette.background, 0.35),
        accent: palette.accent,
        accent_text: contrast_text(palette.accent),
        success: palette.success,
        warning: palette.warning,
        error: palette.error,
        input_bg: palette.surface,
        input_border: palette.border,
        hover_bg: blend(palette.surface, palette.text, emphasis),
        pressed_bg: blend(palette.surface, palette.text, pressed),
        focus_border: palette.accent,
        selected_fill: blend(palette.accent, palette.background, selected),
    }
}

fn contrast_text(color: Color32) -> Color32 {
    if perceived_luminance(color) > 0.6 {
        rgb(0x15110D)
    } else {
        rgb(0xFFF8F0)
    }
}

fn perceived_luminance(color: Color32) -> f32 {
    (0.2126 * f32::from(color.r()) + 0.7152 * f32::from(color.g()) + 0.0722 * f32::from(color.b()))
        / 255.0
}

fn blend(from: Color32, to: Color32, ratio: f32) -> Color32 {
    let ratio = ratio.clamp(0.0, 1.0);
    let mix = |start: u8, end: u8| -> u8 {
        (f32::from(start) + (f32::from(end) - f32::from(start)) * ratio).round() as u8
    };

    Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
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

    #[test]
    fn merges_partial_mode_overrides_into_default_palette() {
        let overrides = parse_appearance_overrides(
            r##"
            [light.colors]
            accent = "#112233"

            [dark.colors]
            text = "#abcdef"
            "##,
        )
        .unwrap();

        let light = ThemeChoice {
            base: BaseMode::Light,
        }
        .tokens(LayoutClass::Standard, &overrides);
        let dark = ThemeChoice {
            base: BaseMode::Dark,
        }
        .tokens(LayoutClass::Standard, &overrides);

        assert_eq!(light.colors.accent, rgb(0x112233));
        assert_eq!(
            light.colors.text_primary,
            base_semantic_palette(BaseMode::Light).text
        );
        assert_eq!(dark.colors.text_primary, rgb(0xABCDEF));
        assert_eq!(
            dark.colors.accent,
            base_semantic_palette(BaseMode::Dark).accent
        );
    }

    #[test]
    fn ignores_invalid_hex_colors() {
        let overrides = parse_appearance_overrides(
            r##"
            [light.colors]
            accent = "#12"
            "##,
        )
        .unwrap();

        let light = ThemeChoice {
            base: BaseMode::Light,
        }
        .tokens(LayoutClass::Standard, &overrides);

        assert_eq!(
            light.colors.accent,
            base_semantic_palette(BaseMode::Light).accent
        );
    }

    #[test]
    fn ignores_unreadable_font_path() {
        let overrides = parse_appearance_overrides(
            r##"
            [fonts]
            ui = "/definitely/missing/font.ttf"
            "##,
        )
        .unwrap();

        assert!(overrides.ui_font.is_none());
    }

    #[test]
    fn keeps_mode_specific_overrides_separate() {
        let overrides = parse_appearance_overrides(
            r##"
            [light.colors]
            background = "#ffffff"

            [dark.colors]
            background = "#000000"
            "##,
        )
        .unwrap();

        let light = ThemeChoice {
            base: BaseMode::Light,
        }
        .tokens(LayoutClass::Standard, &overrides);
        let dark = ThemeChoice {
            base: BaseMode::Dark,
        }
        .tokens(LayoutClass::Standard, &overrides);

        assert_eq!(light.colors.window_bg, rgb(0xFFFFFF));
        assert_eq!(dark.colors.window_bg, rgb(0x000000));
    }

    #[test]
    fn apply_theme_keeps_existing_custom_font_installation() {
        let ctx = Context::default();
        let appearance = AppearanceOverrides {
            light: SemanticPaletteOverride::default(),
            dark: SemanticPaletteOverride::default(),
            ui_font: Some(LoadedFont {
                name: "custom_ui_font".into(),
                bytes: include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf").to_vec(),
            }),
        };

        install_fonts(&ctx, &appearance);
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        for _ in 0..3 {
            apply_theme(
                &ctx,
                ThemeChoice {
                    base: BaseMode::Dark,
                },
                LayoutClass::Standard,
                &appearance,
            );
            let _ = ctx.run(egui::RawInput::default(), |_| {});
        }

        ctx.fonts_mut(|fonts| {
            let definitions = fonts.definitions();
            assert_eq!(
                definitions
                    .families
                    .get(&FontFamily::Monospace)
                    .unwrap()
                    .first(),
                Some(&"custom_ui_font".to_owned())
            );
            assert_eq!(
                definitions
                    .families
                    .get(&FontFamily::Proportional)
                    .unwrap()
                    .first(),
                Some(&"custom_ui_font".to_owned())
            );
        });
    }
}
