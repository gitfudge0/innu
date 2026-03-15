use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    Align, Align2, CentralPanel, ComboBox, Context, Id, Layout, RichText, ScrollArea,
    Ui, Vec2, ViewportCommand, Window,
};
use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::{App, CreationContext};

use crate::backend::nm::{BackendController, WifiController};
use crate::model::{
    AccessPointGroup, AppSnapshot, ConnectRequest, SecurityKind, WifiCommand, WifiEvent,
    signal_bars,
};
use crate::platform::tray::{TrayBridge, TraySignal};
use crate::ui::components::{
    self, AppActionKind, BadgeKind, NetworkRowState, NoticeContent, NoticeKind, field_label,
    modal_shell, modal_title,
};
use crate::ui::theme::{
    LayoutClass, ThemeChoice, ThemeTokens, apply_theme, layout_class_for_width, load_theme_prefs,
    save_theme_prefs,
};

const APP_VERSION_LABEL: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageTone {
    Info,
    Error,
}

#[derive(Debug, Clone)]
struct InlineMessage {
    tone: MessageTone,
    title: String,
    detail: Option<String>,
    retry: Option<ConnectRequest>,
}

#[derive(Debug, Clone)]
struct ConnectDialogState {
    network: AccessPointGroup,
    passphrase: String,
    reveal_password: bool,
}

#[derive(Debug, Clone)]
struct HiddenDialogState {
    ssid: String,
    security: SecurityKind,
    passphrase: String,
    reveal_password: bool,
}

#[derive(Debug, Clone)]
struct ForgetDialogState {
    ssid: String,
    in_use: bool,
}

#[derive(Debug, Clone)]
struct NetworkDetailsState {
    network: AccessPointGroup,
}

#[derive(Debug, Clone, Copy)]
struct NetworkRowMode {
    show_badges: bool,
    action_below: bool,
    action: AppActionKind,
}

pub struct WifiApp {
    controller: Arc<dyn WifiController>,
    snapshot: AppSnapshot,
    tray: Option<TrayBridge>,
    pending_operation: Option<String>,
    refreshing_networks: bool,
    last_snapshot_at: Option<Instant>,
    last_message_at: Option<Instant>,
    inline_message: Option<InlineMessage>,
    connect_dialog: Option<ConnectDialogState>,
    hidden_dialog: Option<HiddenDialogState>,
    forget_dialog: Option<ForgetDialogState>,
    details_dialog: Option<NetworkDetailsState>,
    last_connect_attempt: Option<ConnectRequest>,
    quit_requested: bool,
    theme_choice: ThemeChoice,
    tokens: ThemeTokens,
}

impl WifiApp {
    pub fn new(
        cc: &CreationContext<'_>,
        controller: Arc<BackendController>,
        tray: Option<TrayBridge>,
    ) -> Self {
        let theme_choice = load_theme_prefs();
        let initial_layout = layout_class_for_width(cc.egui_ctx.content_rect().width());
        apply_theme(&cc.egui_ctx, theme_choice, initial_layout);
        let tokens = theme_choice.tokens(initial_layout);

        let controller: Arc<dyn WifiController> = controller;
        let _ = controller.send(WifiCommand::Refresh);

        Self {
            controller,
            snapshot: AppSnapshot::default(),
            tray,
            pending_operation: None,
            refreshing_networks: false,
            last_snapshot_at: None,
            last_message_at: None,
            inline_message: None,
            connect_dialog: None,
            hidden_dialog: None,
            forget_dialog: None,
            details_dialog: None,
            last_connect_attempt: None,
            quit_requested: false,
            theme_choice,
            tokens,
        }
    }

    fn drain_events(&mut self, ctx: &Context) {
        while let Some(event) = self.controller.try_recv() {
            match event {
                WifiEvent::SnapshotUpdated(snapshot) => {
                    self.snapshot = *snapshot;
                    self.last_snapshot_at = Some(Instant::now());
                    self.sync_details_dialog();
                    if let Some(tray) = &self.tray {
                        tray.apply_snapshot(&self.snapshot);
                    }
                    if self.pending_operation.is_none()
                        && let Some(error) = self.snapshot.transient_error.clone()
                    {
                        self.inline_message = Some(InlineMessage {
                            tone: MessageTone::Error,
                            title: error,
                            detail: None,
                            retry: self.last_connect_attempt.clone(),
                        });
                    }
                }
                WifiEvent::OperationStarted(message) => {
                    self.pending_operation = Some(message.clone());
                    if is_refresh_operation(&message) {
                        self.refreshing_networks = true;
                    } else {
                        self.last_message_at = Some(Instant::now());
                        self.inline_message = Some(InlineMessage {
                            tone: MessageTone::Info,
                            title: message,
                            detail: Some("Waiting for NetworkManager.".into()),
                            retry: None,
                        });
                    }
                }
                WifiEvent::OperationFinished(message) => {
                    self.pending_operation = None;
                    if is_refresh_finished(&message) {
                        self.refreshing_networks = false;
                    } else {
                        self.last_message_at = Some(Instant::now());
                        self.inline_message = Some(InlineMessage {
                            tone: MessageTone::Info,
                            title: message,
                            detail: None,
                            retry: None,
                        });
                    }
                }
                WifiEvent::ErrorRaised(message) => {
                    self.pending_operation = None;
                    self.refreshing_networks = false;
                    self.last_message_at = Some(Instant::now());
                    self.inline_message = Some(InlineMessage {
                        tone: MessageTone::Error,
                        title: message,
                        detail: None,
                        retry: self.last_connect_attempt.clone(),
                    });
                }
            }
        }

        if let Some(tray) = &self.tray {
            while let Some(signal) = tray.try_recv() {
                match signal {
                    TraySignal::ShowWindow => {
                        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(ViewportCommand::Focus);
                    }
                    TraySignal::QuitApp => {
                        self.quit_requested = true;
                    }
                }
            }
        }

        if self.pending_operation.is_none()
            && let Some(message_at) = self.last_message_at
            && message_at.elapsed() > Duration::from_secs(5)
            && !matches!(
                self.inline_message.as_ref().map(|message| message.tone),
                Some(MessageTone::Error)
            )
        {
            self.inline_message = None;
        }
    }

    fn send(&self, command: WifiCommand) {
        let _ = self.controller.send(command);
    }

    fn set_error(&mut self, text: impl Into<String>) {
        self.inline_message = Some(InlineMessage {
            tone: MessageTone::Error,
            title: text.into(),
            detail: None,
            retry: self.last_connect_attempt.clone(),
        });
        self.last_message_at = Some(Instant::now());
    }

    fn submit_connect(&mut self, request: ConnectRequest) {
        self.last_connect_attempt = Some(request.clone());
        self.send(WifiCommand::Connect(request));
    }

    fn begin_network_connect(&mut self, network: &AccessPointGroup) {
        if network.in_use {
            return;
        }

        if !network.security.is_supported() {
            self.set_error("Enterprise Wi-Fi is not supported in this version.");
            return;
        }

        if network.security.requires_passphrase() && !network.known {
            self.connect_dialog = Some(ConnectDialogState {
                network: network.clone(),
                passphrase: String::new(),
                reveal_password: false,
            });
            return;
        }

        self.submit_connect(ConnectRequest {
            device_id: network.device_id.clone(),
            ssid: network.ssid.clone(),
            hidden: false,
            security: network.security,
            passphrase: None,
        });
    }

    fn open_hidden_dialog(&mut self) {
        if self.snapshot.primary_device_id.is_none() {
            self.set_error("No managed Wi-Fi adapter is available for joining a hidden network.");
            return;
        }

        self.hidden_dialog = Some(HiddenDialogState {
            ssid: String::new(),
            security: SecurityKind::WpaPsk,
            passphrase: String::new(),
            reveal_password: false,
        });
    }

    fn open_forget_dialog(&mut self, network: &AccessPointGroup) {
        if !network.known {
            return;
        }

        self.forget_dialog = Some(ForgetDialogState {
            ssid: network.ssid.clone(),
            in_use: network.in_use,
        });
    }

    fn open_details_dialog(&mut self, network: &AccessPointGroup) {
        self.details_dialog = Some(NetworkDetailsState {
            network: network.clone(),
        });
    }

    fn sync_details_dialog(&mut self) {
        let Some(dialog) = &mut self.details_dialog else {
            return;
        };

        if let Some(network) = self
            .snapshot
            .visible_networks
            .iter()
            .find(|network| network.ssid == dialog.network.ssid)
        {
            dialog.network = network.clone();
        } else {
            dialog.network.in_use = self
                .snapshot
                .current_connection
                .as_ref()
                .map(|connection| connection.ssid == dialog.network.ssid)
                .unwrap_or(false);
        }
    }

    fn update_theme(
        &mut self,
        ctx: &Context,
        layout: LayoutClass,
        update: impl FnOnce(&mut ThemeChoice),
    ) {
        update(&mut self.theme_choice);
        self.tokens = self.theme_choice.tokens(layout);
        apply_theme(ctx, self.theme_choice, layout);
        let _ = save_theme_prefs(self.theme_choice);
    }

    fn default_status(&self) -> InlineMessage {
        if let Some(connection) = &self.snapshot.current_connection {
            InlineMessage {
                tone: MessageTone::Info,
                title: format!("Connected to {}", connection.ssid),
                detail: Some(format!(
                    "{}  {}  {}",
                    signal_bars(connection.signal),
                    connection.security.label(),
                    connection.band_summary
                )),
                retry: None,
            }
        } else if !self.snapshot.manager_running {
            InlineMessage {
                tone: MessageTone::Error,
                title: "NetworkManager unavailable".into(),
                detail: Some("Start NetworkManager and reopen the app.".into()),
                retry: None,
            }
        } else if !self.snapshot.wifi_available {
            InlineMessage {
                tone: MessageTone::Error,
                title: "No Wi-Fi adapter".into(),
                detail: Some("No managed wireless interface was reported.".into()),
                retry: None,
            }
        } else if self.snapshot.rfkill_blocked {
            InlineMessage {
                tone: MessageTone::Error,
                title: "Wi-Fi blocked by hardware switch".into(),
                detail: Some("Use the hardware radio key or rfkill to unblock wireless.".into()),
                retry: None,
            }
        } else if !self.snapshot.radio_enabled {
            InlineMessage {
                tone: MessageTone::Error,
                title: "Wi-Fi radio is turned off".into(),
                detail: Some("Turn Wi-Fi back on from your system network controls.".into()),
                retry: None,
            }
        } else {
            InlineMessage {
                tone: MessageTone::Info,
                title: "Not connected".into(),
                detail: Some("Choose a nearby network or join a hidden one.".into()),
                retry: None,
            }
        }
    }

    fn render_top_bar(
        &mut self,
        ui: &mut Ui,
        ctx: &Context,
        layout: LayoutClass,
        _available_width: f32,
    ) {
        let mut disconnect_clicked = false;

        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Innu")
                        .size(self.tokens.typography.title)
                        .color(self.tokens.colors.text_primary)
                        .strong(),
                );
                ui.add_space(self.tokens.spacing.tight);
                ui.label(
                    RichText::new(APP_VERSION_LABEL)
                        .size(self.tokens.typography.helper)
                        .color(self.tokens.colors.text_secondary),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.snapshot.current_connection.is_some()
                    && self.inline_message.is_none()
                    && components::primary_button(
                        ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Disconnect,
                    )
                    .clicked()
                {
                    disconnect_clicked = true;
                }

                if components::theme_toggle_button(ui, &self.tokens, layout, self.theme_choice.base)
                    .clicked()
                {
                    self.update_theme(ctx, layout, |theme| theme.toggle_base());
                }
            });
        });

        if disconnect_clicked {
            self.send(WifiCommand::Disconnect);
        }
    }

    fn render_status_block(&mut self, ui: &mut Ui, layout: LayoutClass) {
        let status = self
            .inline_message
            .clone()
            .unwrap_or_else(|| self.default_status());
        let kind = match status.tone {
            MessageTone::Info => NoticeKind::Info,
            MessageTone::Error => NoticeKind::Error,
        };
        let tokens = self.tokens.clone();
        let mut dismiss_clicked = false;
        let mut retry_request = None;
        let (title, emphasis) = split_status_title(&status.title);
        let has_actions = matches!(status.tone, MessageTone::Error);

        if !has_actions {
            ui.vertical(|ui| {
                if let Some(emphasis) = emphasis {
                    ui.label(
                        RichText::new(title)
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
                        RichText::new(title)
                            .size(tokens.typography.section)
                            .color(match status.tone {
                                MessageTone::Info => tokens.colors.text_primary,
                                MessageTone::Error => tokens.colors.error,
                            })
                            .strong(),
                    );
                }

                if let Some(detail) = status.detail.as_deref() {
                    ui.add_space(tokens.spacing.text_stack_gap);
                    ui.label(
                        RichText::new(detail)
                            .size(tokens.typography.helper)
                            .color(tokens.colors.text_secondary),
                    );
                }
            });
            return;
        }

        components::inline_notice(
            ui,
            &tokens,
            layout,
            kind,
            NoticeContent {
                title,
                emphasis,
                detail: status.detail.as_deref(),
            },
            |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if matches!(status.tone, MessageTone::Error) {
                        if components::secondary_button(ui, &tokens, layout, AppActionKind::Dismiss)
                            .clicked()
                        {
                            dismiss_clicked = true;
                        }
                        if let Some(request) = status.retry
                            && components::secondary_button(
                                ui,
                                &tokens,
                                layout,
                                AppActionKind::Retry,
                            )
                            .clicked()
                        {
                            retry_request = Some(request);
                        }
                    }
                });
            },
        );

        if dismiss_clicked {
            self.inline_message = None;
        }
        if let Some(request) = retry_request {
            self.submit_connect(request);
        }
    }

    fn render_networks_header(&mut self, ui: &mut Ui, layout: LayoutClass, _available_width: f32) {
        if layout == LayoutClass::Compact {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Nearby Networks")
                        .size(self.tokens.typography.section)
                        .color(self.tokens.colors.text_primary)
                        .strong(),
                );
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = self.tokens.spacing.inline_cluster_gap;
                    if components::secondary_button(
                        ui,
                        &self.tokens,
                        layout,
                        AppActionKind::JoinHidden,
                    )
                    .clicked()
                    {
                        self.open_hidden_dialog();
                    }
                    if components::secondary_button(
                        ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Refresh,
                    )
                    .clicked()
                    {
                        self.send(WifiCommand::Refresh);
                    }
                });
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Nearby Networks")
                        .size(self.tokens.typography.section)
                        .color(self.tokens.colors.text_primary)
                        .strong(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::secondary_button(
                        ui,
                        &self.tokens,
                        layout,
                        AppActionKind::JoinHidden,
                    )
                    .clicked()
                    {
                        self.open_hidden_dialog();
                    }
                    if components::secondary_button(
                        ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Refresh,
                    )
                    .clicked()
                    {
                        self.send(WifiCommand::Refresh);
                    }
                });
            });
        }
    }

    fn render_networks_body(
        &mut self,
        ui: &mut Ui,
        layout: LayoutClass,
        available_width: f32,
        region_height: f32,
    ) {
        ui.set_min_height(region_height);

        if self.refreshing_networks {
            self.render_list_empty(
                ui,
                "Scanning...",
                "Scanning for access points.",
            );
            return;
        }

        if !self.snapshot.manager_running || !self.snapshot.wifi_available {
            self.render_list_empty(
                ui,
                "Wi-Fi is unavailable",
                "Nearby networks appear here when NetworkManager sees a managed adapter.",
            );
            return;
        }

        if self.snapshot.rfkill_blocked || !self.snapshot.radio_enabled {
            self.render_list_empty(
                ui,
                "Nearby networks are unavailable",
                "The Wi-Fi radio is currently unavailable.",
            );
            return;
        }

        if self.snapshot.visible_networks.is_empty() {
            self.render_list_empty(
                ui,
                "No nearby networks found",
                "Refresh the list or move closer to the access point.",
            );
            return;
        }

        let indicator_height = self.tokens.typography.helper + self.tokens.spacing.tight;
        let (top_indicator_rect, _) = ui.allocate_exact_size(
            Vec2::new(available_width, indicator_height),
            eframe::egui::Sense::hover(),
        );
        let list_height = (ui.available_height() - indicator_height).max(0.0);
        let scroll_output = ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .max_height(list_height)
            .min_scrolled_height(list_height)
            .show(ui, |ui| {
                let networks = self.snapshot.visible_networks.clone();
                for network in networks {
                    self.render_network_row(ui, layout, available_width, &network);
                    ui.add_space(self.tokens.spacing.row_to_row_gap);
                }
            });
        let (bottom_indicator_rect, _) = ui.allocate_exact_size(
            Vec2::new(available_width, indicator_height),
            eframe::egui::Sense::hover(),
        );

        let has_scroll = scroll_output.content_size.y > scroll_output.inner_rect.height() + 1.0;
        if has_scroll {
            let max_offset =
                (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);
            let offset = scroll_output.state.offset.y;
            let show_top = offset > 1.0;
            let show_bottom = offset < max_offset - 1.0;

            ui.painter().text(
                top_indicator_rect.center(),
                Align2::CENTER_CENTER,
                if show_top { "▲" } else { " " },
                eframe::egui::FontId::new(
                    self.tokens.typography.section,
                    eframe::egui::FontFamily::Monospace,
                ),
                self.tokens.colors.text_secondary,
            );

            ui.painter().text(
                bottom_indicator_rect.center(),
                Align2::CENTER_CENTER,
                if show_bottom { "▼" } else { " " },
                eframe::egui::FontId::new(
                    self.tokens.typography.section,
                    eframe::egui::FontFamily::Monospace,
                ),
                self.tokens.colors.text_secondary,
            );
        }
    }

    fn render_list_empty(&self, ui: &mut Ui, title: &str, detail: &str) {
        ui.label(
            RichText::new(title)
                .size(self.tokens.typography.section)
                .color(self.tokens.colors.text_primary)
                .strong(),
        );
        ui.add_space(self.tokens.spacing.text_stack_gap);
        ui.label(
            RichText::new(detail)
                .size(self.tokens.typography.helper)
                .color(self.tokens.colors.text_secondary),
        );
    }

    fn render_network_row(
        &mut self,
        ui: &mut Ui,
        layout: LayoutClass,
        available_width: f32,
        network: &AccessPointGroup,
    ) {
        let row_state = if network.in_use {
            NetworkRowState::InUse
        } else if !network.security.is_supported() {
            NetworkRowState::Unsupported
        } else {
            NetworkRowState::Default
        };
        let row_mode = network_row_mode_for(layout, available_width, network);
        let tokens = self.tokens.clone();
        let metadata = format!(
            "{}  {}  {}",
            signal_bars(network.signal),
            network.security.label(),
            network.band_summary
        );
        let mut connect_clicked = false;
        let mut forget_clicked = false;

        let row_response = components::network_row(ui, &tokens, layout, row_state, |ui| {
            if row_mode.action_below {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&network.ssid)
                                .size(tokens.typography.section)
                                .color(tokens.colors.text_primary)
                                .strong(),
                        );
                    });
                    ui.add_space(tokens.spacing.text_stack_gap);
                    ui.label(
                        RichText::new(&metadata)
                            .size(tokens.typography.helper)
                            .color(tokens.colors.text_secondary),
                    );
                    if row_mode.show_badges {
                        ui.add_space(tokens.spacing.row_internal_gap);
                        render_network_badges(ui, &tokens, network);
                    }
                    ui.add_space(tokens.spacing.control_to_label_gap);
                    ui.scope(|ui| {
                        ui.set_min_height(tokens.spacing.button_height);
                        let enabled = !network.in_use && network.security.is_supported();
                        let response = ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            ui.horizontal(|ui| {
                                if network.known
                                    && components::secondary_button(
                                        ui,
                                        &tokens,
                                        layout,
                                        AppActionKind::Forget,
                                    )
                                    .clicked()
                                {
                                    forget_clicked = true;
                                }

                                ui.add_enabled_ui(enabled, |ui| {
                                    components::primary_button(ui, &tokens, layout, row_mode.action)
                                })
                            })
                        });
                        if enabled && response.inner.inner.inner.clicked() {
                            connect_clicked = true;
                        }
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&network.ssid)
                                .size(tokens.typography.section)
                                .color(tokens.colors.text_primary)
                                .strong(),
                        );
                        ui.add_space(tokens.spacing.text_stack_gap);
                        ui.label(
                            RichText::new(&metadata)
                                .size(tokens.typography.helper)
                                .color(tokens.colors.text_secondary),
                        );
                        if row_mode.show_badges {
                            ui.add_space(tokens.spacing.row_internal_gap);
                            ui.horizontal_wrapped(|ui| {
                                render_network_badges(ui, &tokens, network);
                            });
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if network.known
                            && components::secondary_button(
                                ui,
                                &tokens,
                                layout,
                                AppActionKind::Forget,
                            )
                            .clicked()
                        {
                            forget_clicked = true;
                        }

                        let enabled = !network.in_use && network.security.is_supported();
                        let response = ui.add_enabled_ui(enabled, |ui| {
                            components::primary_button(ui, &tokens, layout, row_mode.action)
                        });
                        if enabled && response.inner.clicked() {
                            connect_clicked = true;
                        }
                    });
                });
            }
        });

        if connect_clicked {
            self.begin_network_connect(network);
        }
        if forget_clicked {
            self.open_forget_dialog(network);
        }
        if row_response.response.clicked() && !connect_clicked && !forget_clicked {
            self.open_details_dialog(network);
        }
    }

    fn render_connect_dialog(&mut self, ctx: &Context, layout: LayoutClass) {
        let Some(dialog) = &mut self.connect_dialog else {
            return;
        };

        let mut keep_open = true;
        let mut request_close = false;
        let mut submit = false;
        let can_submit = passphrase_valid(dialog.network.security, &dialog.passphrase);
        let content_rect = ctx.content_rect();
        let modal_width = modal_width_for(
            layout,
            content_rect.width(),
            self.tokens.spacing.page_padding,
        );
        let modal_max_height = modal_max_height_for(layout, content_rect.height());
        let is_sheet = layout == LayoutClass::Compact;

        Window::new("connect-dialog")
            .id(Id::new("connect-dialog"))
            .anchor(
                if is_sheet {
                    Align2::CENTER_BOTTOM
                } else {
                    Align2::CENTER_CENTER
                },
                [
                    0.0,
                    if is_sheet {
                        -self.tokens.spacing.page_padding
                    } else {
                        0.0
                    },
                ],
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(modal_shell(&self.tokens, layout))
            .min_width(modal_width)
            .max_width(modal_width)
            .max_height(modal_max_height)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                ui.set_max_width(modal_width);
                modal_title(
                    ui,
                    &self.tokens,
                    &format!("Connect to {}", dialog.network.ssid),
                    "Enter the network password.",
                );
                ui.add_space(self.tokens.spacing.section_body_gap);
                field_label(ui, &self.tokens, "PASSWORD");
                ui.add(
                    components::text_field(&mut dialog.passphrase)
                        .password(!dialog.reveal_password),
                );
                ui.add_space(self.tokens.spacing.text_stack_gap);
                ui.checkbox(&mut dialog.reveal_password, "Show password");
                ui.add_space(self.tokens.spacing.section_body_gap);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::primary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::ConfirmConnect,
                    )
                    .clicked()
                        && can_submit
                    {
                        submit = true;
                    }
                    if components::secondary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Cancel,
                    )
                    .clicked()
                    {
                        request_close = true;
                    }
                });
            });

        if request_close {
            keep_open = false;
        }

        if submit {
            if let Some(mut dialog) = self.connect_dialog.take() {
                let passphrase = std::mem::take(&mut dialog.passphrase);
                self.submit_connect(ConnectRequest {
                    device_id: dialog.network.device_id,
                    ssid: dialog.network.ssid,
                    hidden: false,
                    security: dialog.network.security,
                    passphrase: Some(passphrase),
                });
            }
            return;
        }

        if !keep_open && let Some(mut dialog) = self.connect_dialog.take() {
            dialog.passphrase.clear();
        }
    }

    fn render_hidden_dialog(&mut self, ctx: &Context, layout: LayoutClass) {
        let Some(dialog) = &mut self.hidden_dialog else {
            return;
        };

        let mut keep_open = true;
        let mut request_close = false;
        let mut submit = false;
        let can_submit = hidden_form_valid(dialog);
        let content_rect = ctx.content_rect();
        let modal_width = modal_width_for(
            layout,
            content_rect.width(),
            self.tokens.spacing.page_padding,
        );
        let modal_max_height = modal_max_height_for(layout, content_rect.height());
        let is_sheet = layout == LayoutClass::Compact;

        Window::new("hidden-dialog")
            .id(Id::new("hidden-dialog"))
            .anchor(
                if is_sheet {
                    Align2::CENTER_BOTTOM
                } else {
                    Align2::CENTER_CENTER
                },
                [
                    0.0,
                    if is_sheet {
                        -self.tokens.spacing.page_padding
                    } else {
                        0.0
                    },
                ],
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(modal_shell(&self.tokens, layout))
            .min_width(modal_width)
            .max_width(modal_width)
            .max_height(modal_max_height)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                ui.set_max_width(modal_width);
                modal_title(
                    ui,
                    &self.tokens,
                    "Join Hidden Network",
                    "Enter only the details you need.",
                );
                ui.add_space(self.tokens.spacing.section_body_gap);

                field_label(ui, &self.tokens, "NETWORK NAME");
                ui.add(components::text_field(&mut dialog.ssid));
                ui.add_space(self.tokens.spacing.section_body_gap);

                field_label(ui, &self.tokens, "SECURITY");
                ComboBox::from_id_salt("hidden-security")
                    .selected_text(dialog.security.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut dialog.security,
                            SecurityKind::WpaPsk,
                            "WPA Personal",
                        );
                        ui.selectable_value(&mut dialog.security, SecurityKind::Open, "Open");
                    });

                if dialog.security.requires_passphrase() {
                    ui.add_space(self.tokens.spacing.section_body_gap);
                    field_label(ui, &self.tokens, "PASSWORD");
                    ui.add(
                        components::text_field(&mut dialog.passphrase)
                            .password(!dialog.reveal_password),
                    );
                    ui.add_space(self.tokens.spacing.text_stack_gap);
                    ui.checkbox(&mut dialog.reveal_password, "Show password");
                }

                ui.add_space(self.tokens.spacing.section_body_gap);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::primary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::ConfirmConnect,
                    )
                    .clicked()
                        && can_submit
                    {
                        submit = true;
                    }
                    if components::secondary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Cancel,
                    )
                    .clicked()
                    {
                        request_close = true;
                    }
                });
            });

        if request_close {
            keep_open = false;
        }

        if submit {
            let Some(device_id) = self.snapshot.primary_device_id.clone() else {
                self.set_error("No managed Wi-Fi adapter is available for hidden network joins.");
                self.hidden_dialog = None;
                return;
            };

            if let Some(mut dialog) = self.hidden_dialog.take() {
                let ssid = dialog.ssid.trim().to_owned();
                let passphrase = std::mem::take(&mut dialog.passphrase);
                self.submit_connect(ConnectRequest {
                    device_id,
                    ssid,
                    hidden: true,
                    security: dialog.security,
                    passphrase: if dialog.security.requires_passphrase() {
                        Some(passphrase)
                    } else {
                        None
                    },
                });
            }
            return;
        }

        if !keep_open && let Some(mut dialog) = self.hidden_dialog.take() {
            dialog.passphrase.clear();
        }
    }

    fn render_forget_dialog(&mut self, ctx: &Context, layout: LayoutClass) {
        let Some(dialog) = &self.forget_dialog else {
            return;
        };

        let mut keep_open = true;
        let mut request_close = false;
        let mut submit = false;
        let content_rect = ctx.content_rect();
        let modal_width = modal_width_for(
            layout,
            content_rect.width(),
            self.tokens.spacing.page_padding,
        );
        let modal_max_height = modal_max_height_for(layout, content_rect.height());
        let is_sheet = layout == LayoutClass::Compact;
        let title = format!("Forget {}?", dialog.ssid);
        let detail = if dialog.in_use {
            "This will disconnect the current network and remove its saved profile."
        } else {
            "This will remove the saved profile and password for this network."
        };

        Window::new("forget-dialog")
            .id(Id::new("forget-dialog"))
            .anchor(
                if is_sheet {
                    Align2::CENTER_BOTTOM
                } else {
                    Align2::CENTER_CENTER
                },
                [
                    0.0,
                    if is_sheet {
                        -self.tokens.spacing.page_padding
                    } else {
                        0.0
                    },
                ],
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(modal_shell(&self.tokens, layout))
            .min_width(modal_width)
            .max_width(modal_width)
            .max_height(modal_max_height)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                ui.set_max_width(modal_width);
                modal_title(ui, &self.tokens, &title, detail);
                ui.add_space(self.tokens.spacing.section_body_gap);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::primary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Forget,
                    )
                    .clicked()
                    {
                        submit = true;
                    }
                    if components::secondary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Cancel,
                    )
                    .clicked()
                    {
                        request_close = true;
                    }
                });
            });

        if request_close {
            keep_open = false;
        }

        if submit {
            if let Some(dialog) = self.forget_dialog.take() {
                self.send(WifiCommand::Forget(dialog.ssid));
            }
            return;
        }

        if !keep_open {
            self.forget_dialog = None;
        }
    }

    fn render_details_dialog(&mut self, ctx: &Context, layout: LayoutClass) {
        let Some(dialog) = self.details_dialog.clone() else {
            return;
        };

        let mut keep_open = true;
        let mut request_close = false;
        let mut connect_clicked = false;
        let mut disconnect_clicked = false;
        let mut forget_clicked = false;
        let content_rect = ctx.content_rect();
        let modal_width = modal_width_for(
            layout,
            content_rect.width(),
            self.tokens.spacing.page_padding,
        );
        let modal_max_height = modal_max_height_for(layout, content_rect.height());
        let is_sheet = layout == LayoutClass::Compact;
        let network = dialog.network;
        let signal = describe_signal(network.signal);
        let last_scanned = self
            .last_snapshot_at
            .map(format_elapsed)
            .unwrap_or_else(|| "Just now".into());
        let support = if network.security.is_supported() {
            "Supported"
        } else {
            "Unsupported in this version"
        };
        let primary_action = details_primary_action(&network);

        Window::new("details-dialog")
            .id(Id::new("details-dialog"))
            .anchor(
                if is_sheet {
                    Align2::CENTER_BOTTOM
                } else {
                    Align2::CENTER_CENTER
                },
                [
                    0.0,
                    if is_sheet {
                        -self.tokens.spacing.page_padding
                    } else {
                        0.0
                    },
                ],
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(modal_shell(&self.tokens, layout))
            .min_width(modal_width)
            .max_width(modal_width)
            .max_height(modal_max_height)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                ui.set_max_width(modal_width);
                modal_title(
                    ui,
                    &self.tokens,
                    &network.ssid,
                    "Connection details for this network.",
                );
                ui.add_space(self.tokens.spacing.section_body_gap);

                ui.horizontal_wrapped(|ui| {
                    render_network_badges(ui, &self.tokens, &network);
                });

                if network.known || network.in_use || !network.security.is_supported() {
                    ui.add_space(self.tokens.spacing.section_body_gap);
                }

                render_detail_row(ui, &self.tokens, "STATUS", status_label(&network));
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "SECURITY", network.security.label());
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "SIGNAL", &signal);
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "QUALITY", signal_quality_label(network.signal));
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "BAND", &network.band_summary);
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "PROFILE", saved_profile_label(&network));
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "DEVICE", &network.device_id);
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "LAST SCANNED", &last_scanned);
                ui.add_space(self.tokens.spacing.control_to_label_gap);
                render_detail_row(ui, &self.tokens, "SUPPORT", support);

                ui.add_space(self.tokens.spacing.section_body_gap);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::secondary_button(
                        &mut *ui,
                        &self.tokens,
                        layout,
                        AppActionKind::Cancel,
                    )
                    .clicked()
                    {
                        request_close = true;
                    }

                    if network.known
                        && components::secondary_button(
                            &mut *ui,
                            &self.tokens,
                            layout,
                            AppActionKind::Forget,
                        )
                        .clicked()
                    {
                        forget_clicked = true;
                    }

                    if let Some(action) = primary_action {
                        let clicked = if matches!(action, AppActionKind::Disconnect) {
                            components::secondary_button(&mut *ui, &self.tokens, layout, action)
                                .clicked()
                        } else {
                            components::primary_button(&mut *ui, &self.tokens, layout, action)
                                .clicked()
                        };

                        if clicked {
                            match action {
                                AppActionKind::Disconnect => disconnect_clicked = true,
                                _ => connect_clicked = true,
                            }
                        }
                    }
                });
            });

        if request_close {
            keep_open = false;
        }

        if connect_clicked {
            keep_open = false;
            self.begin_network_connect(&network);
        }
        if disconnect_clicked {
            keep_open = false;
            self.send(WifiCommand::Disconnect);
        }
        if forget_clicked {
            keep_open = false;
            self.open_forget_dialog(&network);
        }

        if !keep_open {
            self.details_dialog = None;
        }
    }

    fn render_main_column(
        &mut self,
        ui: &mut Ui,
        ctx: &Context,
        layout: LayoutClass,
        content_width: f32,
    ) {
        self.render_top_bar(ui, ctx, layout, content_width);
        ui.add_space(self.tokens.spacing.status_to_section_gap);
        self.render_status_block(ui, layout);
        ui.add_space(self.tokens.spacing.status_to_section_gap);
        self.render_networks_header(ui, layout, content_width);
        ui.add_space(self.tokens.spacing.section_body_gap);
        let remaining_height = ui.available_height().max(0.0);
        ui.allocate_ui_with_layout(
            Vec2::new(content_width, remaining_height),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_min_height(remaining_height);
                self.render_networks_body(ui, layout, content_width, remaining_height);
            },
        );
    }
}

impl App for WifiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);

        if self.quit_requested {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        let content_width = ctx.content_rect().width();
        let layout = layout_class_for_width(content_width);
        self.tokens = self.theme_choice.tokens(layout);
        apply_theme(ctx, self.theme_choice, layout);

        CentralPanel::default()
            .frame(components::app_frame(&self.tokens))
            .show(ctx, |ui| {
                let available_width = ui.available_width();
                let available_height = ui.available_height();
                let column_width = self.tokens.spacing.max_content_width.min(available_width);
                let gutter = ((available_width - column_width) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    if gutter > 0.0 {
                        ui.add_space(gutter);
                    }
                    ui.vertical(|ui| {
                        ui.set_width(column_width);
                        ui.set_min_height(available_height);
                        self.render_main_column(ui, ctx, layout, column_width);
                    });
                });
            });

        self.render_connect_dialog(ctx, layout);
        self.render_hidden_dialog(ctx, layout);
        self.render_forget_dialog(ctx, layout);
        self.render_details_dialog(ctx, layout);
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.controller.send(WifiCommand::Shutdown);
    }
}

fn passphrase_valid(security: SecurityKind, passphrase: &str) -> bool {
    if !security.requires_passphrase() {
        return true;
    }

    let length = passphrase.chars().count();
    (8..=63).contains(&length)
}

fn hidden_form_valid(dialog: &HiddenDialogState) -> bool {
    !dialog.ssid.trim().is_empty() && passphrase_valid(dialog.security, &dialog.passphrase)
}

fn render_detail_row(ui: &mut Ui, tokens: &ThemeTokens, label: &str, value: &str) {
    field_label(ui, tokens, label);
    ui.add_space(tokens.spacing.text_stack_gap);
    ui.label(
        RichText::new(value)
            .size(tokens.typography.body)
            .color(tokens.colors.text_primary),
    );
}

fn status_label(network: &AccessPointGroup) -> &'static str {
    if network.in_use {
        "Connected"
    } else if network.known {
        "Saved"
    } else {
        "Available"
    }
}

fn describe_signal(signal: Option<u8>) -> String {
    match signal {
        Some(strength) => format!("{}  {}%", signal_bars(Some(strength)), strength),
        None => "Unknown".into(),
    }
}

fn format_elapsed(instant: Instant) -> String {
    let elapsed = instant.elapsed();
    let seconds = elapsed.as_secs();

    match seconds {
        0..=4 => "Just now".into(),
        5..=59 => format!("{}s ago", seconds),
        60..=3599 => format!("{}m ago", seconds / 60),
        _ => format!("{}h ago", seconds / 3600),
    }
}

fn signal_quality_label(signal: Option<u8>) -> &'static str {
    match signal.unwrap_or_default() {
        0..=20 => "Weak",
        21..=40 => "Fair",
        41..=60 => "Good",
        61..=80 => "Very good",
        _ => "Excellent",
    }
}

fn saved_profile_label(network: &AccessPointGroup) -> &'static str {
    if network.known {
        "Saved"
    } else {
        "Not saved"
    }
}

fn details_primary_action(network: &AccessPointGroup) -> Option<AppActionKind> {
    if network.in_use {
        Some(AppActionKind::Disconnect)
    } else if !network.security.is_supported() {
        None
    } else {
        Some(action_for_network(network))
    }
}

fn is_refresh_operation(message: &str) -> bool {
    message == "Refreshing nearby networks"
}

fn is_refresh_finished(message: &str) -> bool {
    message == "Wi-Fi list updated"
}

fn split_status_title(title: &str) -> (&str, Option<&str>) {
    if let Some(ssid) = title.strip_prefix("Connected to ") {
        ("Connected to", Some(ssid))
    } else if let Some(ssid) = title.strip_prefix("Connecting to ") {
        ("Connecting to", Some(ssid))
    } else {
        (title, None)
    }
}

fn network_row_mode_for(
    layout: LayoutClass,
    available_width: f32,
    network: &AccessPointGroup,
) -> NetworkRowMode {
    match layout {
        LayoutClass::Standard => NetworkRowMode {
            show_badges: true,
            action_below: false,
            action: action_for_network(network),
        },
        LayoutClass::Narrow => NetworkRowMode {
            show_badges: available_width >= 420.0,
            action_below: false,
            action: action_for_network(network),
        },
        LayoutClass::Compact => NetworkRowMode {
            show_badges: false,
            action_below: false,
            action: action_for_network(network),
        },
    }
}

fn action_for_network(network: &AccessPointGroup) -> AppActionKind {
    if network.in_use {
        AppActionKind::Connected
    } else if !network.security.is_supported() {
        AppActionKind::Unsupported
    } else if network.known {
        AppActionKind::Reconnect
    } else if network.security.requires_passphrase() {
        AppActionKind::EnterPassword
    } else {
        AppActionKind::Connect
    }
}

fn render_network_badges(ui: &mut Ui, tokens: &ThemeTokens, network: &AccessPointGroup) {
    if network.known {
        components::badge(ui, tokens, BadgeKind::Neutral, "SAVED");
    }
    if network.in_use {
        components::badge(ui, tokens, BadgeKind::Active, "IN USE");
    }
    if !network.security.is_supported() {
        components::badge(ui, tokens, BadgeKind::Warning, "UNSUPPORTED");
    }
}

fn modal_width_for(layout: LayoutClass, available_width: f32, page_padding: f32) -> f32 {
    let inner = (available_width - 2.0 * page_padding).max(200.0);
    match layout {
        LayoutClass::Standard => inner.min(420.0),
        LayoutClass::Narrow | LayoutClass::Compact => inner,
    }
}

fn modal_max_height_for(layout: LayoutClass, available_height: f32) -> f32 {
    match layout {
        LayoutClass::Compact => available_height * 0.75,
        LayoutClass::Narrow | LayoutClass::Standard => available_height * 0.85,
    }
}
