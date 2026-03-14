use eframe::egui::{
    self, Align, Button, Color32, CursorIcon, FontFamily, Frame, Layout, Margin, Response,
    RichText, Stroke, TextEdit, Ui, Vec2,
};

use crate::ui::theme::{BaseMode, LayoutClass, ThemeTokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeKind {
    Neutral,
    Active,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkRowState {
    Default,
    InUse,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    ThemeLight,
    ThemeDark,
    Refresh,
    Hidden,
    Disconnect,
    Connect,
    Password,
    Connected,
    Unsupported,
    Retry,
    Close,
    Confirm,
    Forget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppActionKind {
    Refresh,
    JoinHidden,
    Disconnect,
    Connect,
    Reconnect,
    EnterPassword,
    Connected,
    Unsupported,
    Retry,
    Dismiss,
    Cancel,
    ConfirmConnect,
    Forget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionVisual {
    Text(&'static str),
    Icon(AppIcon),
}

#[derive(Debug, Clone, Copy)]
pub struct NoticeContent<'a> {
    pub title: &'a str,
    pub emphasis: Option<&'a str>,
    pub detail: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
struct ButtonStyle {
    fill: Color32,
    stroke: Color32,
    text: Color32,
    min_width: f32,
    strong: bool,
}

pub fn app_frame(tokens: &ThemeTokens) -> Frame {
    Frame::new()
        .fill(tokens.colors.window_bg)
        .inner_margin(Margin::same(tokens.spacing.page_padding as i8))
}

pub fn modal_shell(tokens: &ThemeTokens, _layout: LayoutClass) -> Frame {
    Frame::new()
        .fill(tokens.colors.panel_bg)
        .stroke(Stroke::new(tokens.strokes.standard, tokens.colors.border))
        .inner_margin(Margin::same(tokens.spacing.standard as i8))
}

pub fn panel(tokens: &ThemeTokens, subtle: bool) -> Frame {
    Frame::new()
        .fill(if subtle {
            tokens.colors.elevated_panel_bg
        } else {
            tokens.colors.panel_bg
        })
        .stroke(Stroke::new(tokens.strokes.standard, tokens.colors.border))
        .inner_margin(Margin::same(tokens.spacing.standard as i8))
}

pub fn theme_toggle_button(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    mode: BaseMode,
) -> Response {
    let target_mode = match mode {
        BaseMode::Light => BaseMode::Dark,
        BaseMode::Dark => BaseMode::Light,
    };
    let response = ui.add(
        Button::new(button_rich_text(
            tokens,
            layout,
            theme_toggle_visual(layout, mode),
            tokens.colors.text_primary,
            true,
        ))
        .fill(tokens.colors.window_bg)
        .stroke(Stroke::new(tokens.strokes.standard, tokens.colors.border))
        .min_size(Vec2::new(
            theme_toggle_width(layout),
            theme_toggle_height(layout),
        )),
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(theme_toggle_tooltip(target_mode))
}

pub fn primary_button(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    action: AppActionKind,
) -> Response {
    button_with_visual(
        ui,
        tokens,
        layout,
        action_visual_for(layout, action),
        action_label(action),
        ButtonStyle {
            fill: tokens.colors.accent,
            stroke: tokens.colors.accent,
            text: tokens.colors.accent_text,
            min_width: primary_button_width(layout),
            strong: true,
        },
    )
}

pub fn secondary_button(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    action: AppActionKind,
) -> Response {
    button_with_visual(
        ui,
        tokens,
        layout,
        action_visual_for(layout, action),
        action_label(action),
        ButtonStyle {
            fill: tokens.colors.window_bg,
            stroke: tokens.colors.border,
            text: tokens.colors.text_primary,
            min_width: secondary_button_width(tokens, layout),
            strong: false,
        },
    )
}

pub fn action_visual_for(layout: LayoutClass, action: AppActionKind) -> ActionVisual {
    match layout {
        LayoutClass::Standard => ActionVisual::Text(action_label(action)),
        LayoutClass::Narrow | LayoutClass::Compact => ActionVisual::Icon(action_icon(action)),
    }
}

pub fn inline_notice<R>(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    kind: NoticeKind,
    content: NoticeContent<'_>,
    add_actions: impl FnOnce(&mut Ui) -> R,
) -> R {
    let text_color = match kind {
        NoticeKind::Info => tokens.colors.text_primary,
        NoticeKind::Error => tokens.colors.error,
    };

    let content = |ui: &mut Ui| {
        if let Some(emphasis) = content.emphasis {
            ui.label(
                RichText::new(content.title)
                    .size(tokens.typography.helper)
                    .color(tokens.colors.text_secondary),
            );
            ui.add_space(tokens.spacing.text_stack_gap);
            ui.label(
                RichText::new(emphasis)
                    .size(tokens.typography.title)
                    .color(tokens.colors.text_primary)
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new(content.title)
                    .size(tokens.typography.section)
                    .color(text_color)
                    .strong(),
            );
        }

        if let Some(detail) = content.detail {
            ui.add_space(tokens.spacing.text_stack_gap);
            ui.label(
                RichText::new(detail)
                    .size(tokens.typography.helper)
                    .color(tokens.colors.text_secondary),
            );
        }
    };

    match layout {
        LayoutClass::Compact => {
            ui.vertical(|ui| {
                ui.vertical(content);
                ui.add_space(tokens.spacing.control_to_label_gap);
                ui.with_layout(Layout::right_to_left(Align::Center), add_actions)
                    .inner
            })
            .inner
        }
        LayoutClass::Narrow | LayoutClass::Standard => {
            ui.horizontal(|ui| {
                ui.vertical(content);
                ui.with_layout(Layout::right_to_left(Align::TOP), add_actions)
                    .inner
            })
            .inner
        }
    }
}

pub fn badge(ui: &mut Ui, tokens: &ThemeTokens, kind: BadgeKind, label: &str) {
    let (fill, text) = match kind {
        BadgeKind::Neutral => (
            tokens.colors.elevated_panel_bg,
            tokens.colors.text_secondary,
        ),
        BadgeKind::Active => (
            tokens.colors.selected_fill,
            if tokens.colors.selected_fill == tokens.colors.accent {
                tokens.colors.accent_text
            } else {
                tokens.colors.text_primary
            },
        ),
        BadgeKind::Warning => (tokens.colors.warning, tokens.colors.window_bg),
    };

    Frame::new()
        .fill(fill)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(tokens.typography.micro)
                    .color(text)
                    .strong(),
            );
        });
}

pub fn network_row<R>(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    state: NetworkRowState,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let fill = match state {
        NetworkRowState::Default => Color32::TRANSPARENT,
        NetworkRowState::InUse => tokens.colors.selected_fill,
        NetworkRowState::Unsupported => Color32::TRANSPARENT,
    };

    let hover_fill = match state {
        NetworkRowState::Default => tokens.colors.hover_bg,
        NetworkRowState::InUse => tokens.colors.hover_bg,
        NetworkRowState::Unsupported => tokens.colors.hover_bg,
    };

    let mut response = Frame::new()
        .fill(fill)
        .inner_margin(Margin::symmetric(
            0,
            match layout {
                LayoutClass::Standard => 6,
                LayoutClass::Narrow => 4,
                LayoutClass::Compact => 2,
            },
        ))
        .show(ui, add_contents);

    response.response = response.response.interact(egui::Sense::click());
    response.response = response.response.on_hover_cursor(CursorIcon::PointingHand);

    if response.response.hovered() {
        ui.painter()
            .rect_filled(response.response.rect, 0, hover_fill.gamma_multiply(0.18));
        ui.painter().rect_stroke(
            response.response.rect,
            0,
            Stroke::new(tokens.strokes.standard, tokens.colors.focus_border),
            egui::StrokeKind::Outside,
        );
    }

    response
}

pub fn modal_title(ui: &mut Ui, tokens: &ThemeTokens, title: &str, detail: &str) {
    ui.label(
        RichText::new(title)
            .size(tokens.typography.section)
            .color(tokens.colors.text_primary)
            .strong(),
    );
    ui.add_space(tokens.spacing.text_stack_gap);
    ui.label(
        RichText::new(detail)
            .size(tokens.typography.helper)
            .color(tokens.colors.text_secondary),
    );
}

pub fn field_label(ui: &mut Ui, tokens: &ThemeTokens, label: &str) {
    ui.label(
        RichText::new(label)
            .size(tokens.typography.helper)
            .color(tokens.colors.text_secondary),
    );
}

pub fn text_field<'a>(text: &'a mut String) -> TextEdit<'a> {
    TextEdit::singleline(text).desired_width(f32::INFINITY)
}

fn button_with_visual(
    ui: &mut Ui,
    tokens: &ThemeTokens,
    layout: LayoutClass,
    visual: ActionVisual,
    tooltip: &'static str,
    style: ButtonStyle,
) -> Response {
    let response = ui.add(
        Button::new(button_rich_text(
            tokens,
            layout,
            visual,
            style.text,
            style.strong,
        ))
        .fill(style.fill)
        .stroke(Stroke::new(tokens.strokes.standard, style.stroke))
        .min_size(Vec2::new(style.min_width, tokens.spacing.button_height)),
    );

    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    match visual {
        ActionVisual::Icon(_) => response.on_hover_text(tooltip),
        ActionVisual::Text(_) => response,
    }
}

fn button_rich_text(
    tokens: &ThemeTokens,
    layout: LayoutClass,
    visual: ActionVisual,
    color: Color32,
    strong: bool,
) -> RichText {
    match visual {
        ActionVisual::Text(label) => {
            let text = RichText::new(label)
                .size(tokens.typography.body)
                .color(color);
            if strong {
                text.strong()
            } else {
                text
            }
        }
        ActionVisual::Icon(icon) => icon_rich_text(tokens, layout, icon, color),
    }
}

fn icon_rich_text(
    tokens: &ThemeTokens,
    layout: LayoutClass,
    icon: AppIcon,
    color: Color32,
) -> RichText {
    let size = match layout {
        LayoutClass::Standard => tokens.typography.body,
        LayoutClass::Narrow => tokens.typography.section,
        LayoutClass::Compact => tokens.typography.body + 2.0,
    };

    RichText::new(icon_ligature(icon))
        .family(FontFamily::Name("material_symbols_sharp".into()))
        .size(size)
        .color(color)
}

fn theme_toggle_width(layout: LayoutClass) -> f32 {
    match layout {
        LayoutClass::Standard => 84.0,
        LayoutClass::Narrow | LayoutClass::Compact => 32.0,
    }
}

fn theme_toggle_height(layout: LayoutClass) -> f32 {
    match layout {
        LayoutClass::Standard => 28.0,
        LayoutClass::Narrow => 28.0,
        LayoutClass::Compact => 26.0,
    }
}

fn primary_button_width(layout: LayoutClass) -> f32 {
    match layout {
        LayoutClass::Standard => 120.0,
        LayoutClass::Narrow | LayoutClass::Compact => 32.0,
    }
}

fn secondary_button_width(tokens: &ThemeTokens, layout: LayoutClass) -> f32 {
    match layout {
        LayoutClass::Standard => 0.0,
        LayoutClass::Narrow | LayoutClass::Compact => tokens.spacing.button_height,
    }
}

fn theme_toggle_visual(layout: LayoutClass, mode: BaseMode) -> ActionVisual {
    match layout {
        LayoutClass::Standard => ActionVisual::Text(match mode {
            BaseMode::Light => "LIGHT",
            BaseMode::Dark => "DARK",
        }),
        LayoutClass::Narrow | LayoutClass::Compact => ActionVisual::Icon(match mode {
            BaseMode::Light => AppIcon::ThemeLight,
            BaseMode::Dark => AppIcon::ThemeDark,
        }),
    }
}

fn theme_toggle_tooltip(target_mode: BaseMode) -> &'static str {
    match target_mode {
        BaseMode::Light => "Switch to light theme",
        BaseMode::Dark => "Switch to dark theme",
    }
}

fn action_label(action: AppActionKind) -> &'static str {
    match action {
        AppActionKind::Refresh => "REFRESH",
        AppActionKind::JoinHidden => "HIDDEN",
        AppActionKind::Disconnect => "DISCONNECT",
        AppActionKind::Connect => "CONNECT",
        AppActionKind::Reconnect => "RECONNECT",
        AppActionKind::EnterPassword => "ENTER PASSWORD",
        AppActionKind::Connected => "CONNECTED",
        AppActionKind::Unsupported => "UNSUPPORTED",
        AppActionKind::Retry => "RETRY",
        AppActionKind::Dismiss => "DISMISS",
        AppActionKind::Cancel => "CANCEL",
        AppActionKind::ConfirmConnect => "CONNECT",
        AppActionKind::Forget => "FORGET",
    }
}

fn action_icon(action: AppActionKind) -> AppIcon {
    match action {
        AppActionKind::Refresh => AppIcon::Refresh,
        AppActionKind::JoinHidden => AppIcon::Hidden,
        AppActionKind::Disconnect => AppIcon::Disconnect,
        AppActionKind::Connect | AppActionKind::Reconnect => AppIcon::Connect,
        AppActionKind::EnterPassword => AppIcon::Password,
        AppActionKind::Connected => AppIcon::Connected,
        AppActionKind::Unsupported => AppIcon::Unsupported,
        AppActionKind::Retry => AppIcon::Retry,
        AppActionKind::Dismiss | AppActionKind::Cancel => AppIcon::Close,
        AppActionKind::ConfirmConnect => AppIcon::Confirm,
        AppActionKind::Forget => AppIcon::Forget,
    }
}

fn icon_ligature(icon: AppIcon) -> &'static str {
    match icon {
        AppIcon::ThemeLight => "\u{e51c}",
        AppIcon::ThemeDark => "\u{e518}",
        AppIcon::Refresh => "\u{e5d5}",
        AppIcon::Hidden => "\u{e145}",
        AppIcon::Disconnect => "\u{e16f}",
        AppIcon::Connect => "\u{ea77}",
        AppIcon::Password => "\u{e73c}",
        AppIcon::Connected => "\u{e5ca}",
        AppIcon::Unsupported => "\u{f08c}",
        AppIcon::Retry => "\u{e5d5}",
        AppIcon::Close => "\u{e5cd}",
        AppIcon::Confirm => "\u{e5ca}",
        AppIcon::Forget => "\u{e92b}",
    }
}
