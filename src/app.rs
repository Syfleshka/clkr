//! The eframe application: state, event handling, persistence and the frame layout.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use egui::{
    Align, Align2, Color32, Layout, Margin, Rect, RichText, Stroke, Theme, UiBuilder, Vec2, ViewportCommand, pos2, vec2,
};

use crate::engine::{AppEvent, Controller, StopReason};
use crate::hotkeys::{CaptureTarget, Hotkeys};
use crate::i18n::{Tr, tr};
use crate::platform::{self, PickKind, PickRequest, PickResult};
use crate::settings::{AppConfig, HotkeyConfig, Problem, SeqPoint, Settings, ThemeChoice};
use crate::ui::icons;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets::{self, ButtonKind};

pub const FULL_SIZE: Vec2 = vec2(470.0, 780.0);
pub const MIN_SIZE: Vec2 = vec2(430.0, 560.0);
pub const COMPACT_SIZE: Vec2 = vec2(380.0, 116.0);
pub const TITLE_H: f32 = 46.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Warning,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub born: f64,
}

pub enum Dialog {
    Rename { index: usize, text: String },
    Delete { index: usize },
}

/// Measures the real click rate from the engine's counter.
#[derive(Default)]
pub struct CpsMeter {
    samples: VecDeque<(f64, u64)>,
    pub history: VecDeque<f32>,
    last_push: f64,
    pub cps: f32,
}

impl CpsMeter {
    pub const HISTORY: usize = 90;

    fn update(&mut self, now: f64, clicks: u64, running: bool) {
        if !running {
            self.samples.clear();
            self.cps = 0.0;
            return;
        }
        if self.samples.back().is_some_and(|&(_, c)| c > clicks) {
            self.samples.clear(); // a new run started
        }
        self.samples.push_back((now, clicks));
        while self.samples.len() > 2 && now - self.samples[1].0 > 1.0 {
            self.samples.pop_front();
        }
        if let (Some(&(t0, c0)), Some(&(t1, c1))) = (self.samples.front(), self.samples.back())
            && t1 - t0 >= 0.2
        {
            self.cps = ((c1 - c0) as f64 / (t1 - t0)) as f32;
        }
        if now - self.last_push >= 0.1 {
            self.last_push = now;
            self.history.push_back(self.cps);
            while self.history.len() > Self::HISTORY {
                self.history.pop_front();
            }
        }
    }
}

#[derive(Default)]
struct Applied {
    theme: Option<ThemeChoice>,
    on_top: Option<bool>,
    compact: Option<bool>,
}

pub struct App {
    pub cfg: AppConfig,
    cfg_path: PathBuf,
    saved: AppConfig,
    dirty_since: Option<f64>,
    pub controller: Arc<Controller>,
    pub hotkeys: Arc<Hotkeys>,
    events_rx: Receiver<AppEvent>,
    events_tx: Sender<AppEvent>,
    pub pal: Palette,
    pub grad_idle: egui::TextureHandle,
    pub grad_run: egui::TextureHandle,
    pub capture: Option<CaptureTarget>,
    pub picking: bool,
    pub meter: CpsMeter,
    pub toasts: Vec<Toast>,
    pub dialog: Option<Dialog>,
    applied: Applied,
    pushed: Option<(Settings, HotkeyConfig, u32, bool)>,
    full_size: Vec2,
    was_active: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: AppConfig, cfg_path: PathBuf) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::install_fonts(&ctx);
        ctx.set_style_of(Theme::Dark, theme::style(&Palette::DARK));
        ctx.set_style_of(Theme::Light, theme::style(&Palette::LIGHT));

        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = cc.window_handle()
                && let RawWindowHandle::Win32(h) = handle.as_raw()
            {
                platform::init_main_window(h.hwnd.get());
            }
        }
        if let Some(pos) = cfg.window_pos {
            platform::restore_window_position(pos);
        }

        let (events_tx, events_rx) = mpsc::channel();
        let controller = Controller::new(ctx.clone(), events_tx.clone());
        controller.stats.set_total(cfg.total_clicks);
        let hotkeys = Hotkeys::spawn(controller.clone(), events_tx.clone(), ctx.clone());

        let pal = Palette::get(cfg.theme);
        let (grad_idle, grad_run) = Self::gradients(&ctx, &pal);
        Self {
            saved: cfg.clone(),
            cfg,
            cfg_path,
            dirty_since: None,
            controller,
            hotkeys,
            events_rx,
            events_tx,
            pal,
            grad_idle,
            grad_run,
            capture: None,
            picking: false,
            meter: CpsMeter::default(),
            toasts: Vec::new(),
            dialog: None,
            applied: Applied::default(),
            pushed: None,
            full_size: FULL_SIZE,
            was_active: false,
        }
    }

    fn gradients(ctx: &egui::Context, pal: &Palette) -> (egui::TextureHandle, egui::TextureHandle) {
        (
            theme::gradient(ctx, "grad-idle", pal.accent, pal.accent2),
            theme::gradient(ctx, "grad-run", pal.danger, pal.danger2),
        )
    }

    pub fn t(&self) -> &'static Tr {
        tr(self.cfg.language)
    }

    pub fn toast(&mut self, text: impl Into<String>, kind: ToastKind, now: f64) {
        self.toasts.push(Toast { text: text.into(), kind, born: now });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn problem_text(&self, p: Problem) -> &'static str {
        let t = self.t();
        match p {
            Problem::NoKey => t.problem_no_key,
            Problem::KeyIsHotkey => t.problem_key_is_hotkey,
            Problem::NoPoints => t.problem_no_points,
            Problem::EmptyArea => t.problem_empty_area,
        }
    }

    pub fn begin_capture(&mut self, target: CaptureTarget) {
        if self.capture == Some(target) {
            self.capture = None;
            self.hotkeys.cancel_capture();
        } else {
            self.capture = Some(target);
            self.hotkeys.begin_capture(target);
        }
    }

    pub fn start_pick(&mut self, kind: PickKind, ctx: &egui::Context) {
        if self.picking {
            return;
        }
        self.picking = true;
        let t = self.t();
        let hint = match kind {
            PickKind::Point => t.pick_hint_point,
            PickKind::Area => t.pick_hint_area,
            PickKind::Points => t.pick_hint_points,
        }
        .to_owned();
        let existing = match kind {
            PickKind::Points => self.cfg.settings().sequence.iter().map(|p| (p.x, p.y)).collect(),
            _ => Vec::new(),
        };
        let tx = self.events_tx.clone();
        let ctx = ctx.clone();
        platform::pick_on_screen(PickRequest { kind, hint, existing }, move |result| {
            let _ = tx.send(AppEvent::Picked(result));
            ctx.request_repaint();
        });
    }

    fn new_point(&self, x: i32, y: i32) -> SeqPoint {
        let s = self.cfg.settings();
        SeqPoint { x, y, button: s.button, click: s.click, pause_ms: 0 }
    }

    fn handle_events(&mut self, now: f64) {
        while let Ok(event) = self.events_rx.try_recv() {
            let t = self.t();
            match event {
                AppEvent::Stopped(StopReason::LimitReached) => self.toast(t.stopped_limit, ToastKind::Success, now),
                AppEvent::Stopped(StopReason::MouseMoved) => self.toast(t.stopped_mouse, ToastKind::Warning, now),
                AppEvent::Stopped(StopReason::User) => {}
                AppEvent::StartRefused(p) => self.toast(self.problem_text(p), ToastKind::Warning, now),
                AppEvent::AddPoint(x, y) => {
                    let point = self.new_point(x, y);
                    let seq = &mut self.cfg.settings_mut().sequence;
                    seq.push(point);
                    let n = seq.len();
                    self.toast(format!("{} #{n}  ({x}, {y})", t.point_added), ToastKind::Success, now);
                }
                AppEvent::Captured(target, combo) => {
                    self.capture = None;
                    if let Some(combo) = combo {
                        match target {
                            CaptureTarget::Toggle => self.cfg.hotkeys.toggle = combo,
                            CaptureTarget::AddPoint => self.cfg.hotkeys.add_point = combo,
                            CaptureTarget::ActionKey => self.cfg.settings_mut().key = combo,
                        }
                    }
                }
                AppEvent::Picked(result) => {
                    self.picking = false;
                    match result {
                        PickResult::Point(x, y) => self.cfg.settings_mut().point = (x, y),
                        PickResult::Area(r) => self.cfg.settings_mut().area = r,
                        PickResult::Points(points) => {
                            let n = points.len() as u64;
                            let new: Vec<SeqPoint> = points.into_iter().map(|(x, y)| self.new_point(x, y)).collect();
                            self.cfg.settings_mut().sequence.extend(new);
                            self.toast(format!("{} {n} {}", t.points_added, t.points_word(n)), ToastKind::Success, now);
                        }
                        PickResult::Cancelled => {}
                    }
                }
            }
        }
    }

    fn apply_window_state(&mut self, ctx: &egui::Context) {
        if self.applied.theme != Some(self.cfg.theme) {
            self.applied.theme = Some(self.cfg.theme);
            ctx.set_theme(match self.cfg.theme {
                ThemeChoice::Dark => Theme::Dark,
                ThemeChoice::Light => Theme::Light,
            });
            self.pal = Palette::get(self.cfg.theme);
            (self.grad_idle, self.grad_run) = Self::gradients(ctx, &self.pal);
        }
        if self.applied.on_top != Some(self.cfg.always_on_top) {
            self.applied.on_top = Some(self.cfg.always_on_top);
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if self.cfg.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        }
        if self.applied.compact != Some(self.cfg.compact) {
            let first = self.applied.compact.is_none();
            self.applied.compact = Some(self.cfg.compact);
            if self.cfg.compact {
                if !first {
                    self.full_size = ctx.content_rect().size();
                }
                ctx.send_viewport_cmd(ViewportCommand::Resizable(false));
                ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(COMPACT_SIZE));
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(COMPACT_SIZE));
            } else {
                ctx.send_viewport_cmd(ViewportCommand::Resizable(true));
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(self.full_size));
                ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(MIN_SIZE));
            }
        }
    }

    /// Push changed settings to the engine / hotkey thread and save to disk.
    fn sync(&mut self, ctx: &egui::Context, now: f64) {
        let current = (self.cfg.settings().clone(), self.cfg.hotkeys, self.cfg.start_delay_s, self.cfg.failsafe);
        if self.pushed.as_ref() != Some(&current) {
            self.controller.update_live(&current.0, &current.1, current.2, current.3);
            self.hotkeys.set_config(current.1);
            self.pushed = Some(current);
        }

        if self.cfg != self.saved {
            let since = *self.dirty_since.get_or_insert(now);
            if now - since > 0.6 {
                self.save();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(700));
            }
        } else {
            self.dirty_since = None;
        }
    }

    fn save(&mut self) {
        self.cfg.total_clicks = self.controller.stats.total();
        if !self.cfg.compact
            && let Some(pos) = platform::window_position()
        {
            self.cfg.window_pos = Some(pos);
        }
        let _ = self.cfg.save(&self.cfg_path);
        self.saved = self.cfg.clone();
        self.dirty_since = None;
    }

    pub fn config_path(&self) -> &std::path::Path {
        &self.cfg_path
    }
}

impl eframe::App for App {
    /// Also runs while the window is minimised or hidden (e.g. during on-screen picking).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        self.handle_events(now);
        // Persist the lifetime counter whenever a run ends.
        let active = self.controller.state().is_active();
        if self.was_active && !active {
            self.save();
        }
        self.was_active = active;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ui.input(|i| i.time);
        self.apply_window_state(&ctx);

        let state = self.controller.state();
        self.meter.update(now, self.controller.stats.clicks(), state.is_active());
        if state.is_active() || self.capture.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }

        let pal = self.pal;
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, pal.bg);

        let title = Rect::from_min_size(full.min, vec2(full.width(), TITLE_H));
        ui.scope_builder(UiBuilder::new().max_rect(title), |ui| self.title_bar(ui));

        let body = Rect::from_min_max(pos2(full.min.x + 14.0, title.max.y), pos2(full.max.x - 14.0, full.max.y - 14.0));
        ui.scope_builder(UiBuilder::new().max_rect(body).layout(Layout::top_down(Align::Min)), |ui| {
            if self.cfg.compact {
                self.compact_view(ui);
            } else {
                self.hero(ui);
                ui.add_space(12.0);
                self.tabs(ui);
                ui.add_space(10.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    match self.cfg.tab {
                        crate::settings::Tab::Click => self.tab_click(ui),
                        crate::settings::Tab::Location => self.tab_location(ui),
                        crate::settings::Tab::Settings => self.tab_settings(ui),
                    }
                    ui.add_space(4.0);
                });
            }
        });

        self.edge_resize(&ctx);
        self.show_toasts(&ctx, now);
        self.show_dialog(&ctx);
        self.sync(&ctx, now);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.controller.stop();
        // Give the engine a moment to finish a click so no button is left pressed.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
        while self.controller.state().is_active() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        self.save();
    }
}

impl App {
    /// The window has no system frame, so resizing from its edges is done here.
    fn edge_resize(&self, ctx: &egui::Context) {
        use egui::{CursorIcon, ResizeDirection};
        const GRIP: f32 = 6.0;
        if self.cfg.compact || self.dialog.is_some() {
            return;
        }
        let rect = ctx.content_rect();
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else { return };
        let (left, right) = (pos.x - rect.left() < GRIP, rect.right() - pos.x < GRIP);
        let (top, bottom) = (pos.y - rect.top() < GRIP, rect.bottom() - pos.y < GRIP);
        let (dir, cursor) = match (left, right, top, bottom) {
            (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest),
            (_, true, true, _) => (ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast),
            (true, _, _, true) => (ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest),
            (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast),
            (true, ..) => (ResizeDirection::West, CursorIcon::ResizeWest),
            (_, true, ..) => (ResizeDirection::East, CursorIcon::ResizeEast),
            (_, _, true, _) => (ResizeDirection::North, CursorIcon::ResizeNorth),
            (_, _, _, true) => (ResizeDirection::South, CursorIcon::ResizeSouth),
            _ => return,
        };
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        use crate::settings::Tab;
        let t = self.t();
        let labels = [
            format!("{}  {}", icons::LIGHTNING, t.tab_click),
            format!("{}  {}", icons::TARGET, t.tab_location),
            format!("{}  {}", icons::GEAR_SIX, t.tab_settings),
        ];
        let items = [
            (Tab::Click, labels[0].as_str()),
            (Tab::Location, labels[1].as_str()),
            (Tab::Settings, labels[2].as_str()),
        ];
        widgets::segmented(ui, &self.pal, &mut self.cfg.tab, &items, true);
    }

    fn show_toasts(&mut self, ctx: &egui::Context, now: f64) {
        const LIFE: f64 = 3.2;
        self.toasts.retain(|t| now - t.born < LIFE);
        if self.toasts.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
        let pal = self.pal;
        let mut offset = 18.0;
        for (i, toast) in self.toasts.iter().rev().enumerate().take(2) {
            let age = now - toast.born;
            let alpha = ((age / 0.18).min(1.0) * ((LIFE - age) / 0.35).min(1.0)) as f32;
            let slide = (1.0 - (age / 0.18).min(1.0) as f32) * 12.0;
            let (icon, color) = match toast.kind {
                ToastKind::Success => (icons::CHECK, pal.success),
                ToastKind::Warning => (icons::WARNING, pal.warning),
            };
            let area = egui::Area::new(egui::Id::unique(("toast", i)))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -offset + slide))
                .interactable(false);
            let r = area.show(ctx, |ui| {
                ui.set_opacity(alpha);
                egui::Frame::new()
                    .fill(if pal.dark { Color32::from_rgb(34, 38, 52) } else { Color32::from_rgb(30, 33, 45) })
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(18)))
                    .corner_radius(12)
                    .inner_margin(Margin::symmetric(14, 9))
                    .shadow(egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(90) })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).size(15.0).color(color));
                            ui.label(RichText::new(&toast.text).size(13.0).color(Color32::from_rgb(236, 238, 244)));
                        });
                    });
            });
            offset += r.response.rect.height() + 8.0;
        }
    }

    fn show_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.dialog else { return };
        let pal = self.pal;
        let t = tr(self.cfg.language);
        let mut close = false;
        let mut apply = false;
        let frame = egui::Frame::new()
            .fill(pal.surface)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(16)
            .inner_margin(Margin::same(20))
            .shadow(ctx.global_style().visuals.window_shadow);
        let modal = egui::Modal::new(egui::Id::unique("dialog"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(if pal.dark { 150 } else { 90 }))
            .show(ctx, |ui| {
                ui.set_width(300.0);
                match dialog {
                    Dialog::Rename { text, .. } => {
                        ui.label(RichText::new(t.rename_title).font(theme::semibold(16.0)));
                        ui.add_space(6.0);
                        let edit = ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(f32::INFINITY)
                                .margin(Margin::symmetric(10, 7))
                                .char_limit(40),
                        );
                        if !edit.has_focus() && !edit.lost_focus() {
                            edit.request_focus();
                        }
                        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            apply = true;
                        }
                    }
                    Dialog::Delete { index } => {
                        ui.label(RichText::new(t.delete_title).font(theme::semibold(16.0)));
                        ui.add_space(2.0);
                        let name = self.cfg.profiles.get(*index).map(|p| p.name.as_str()).unwrap_or_default();
                        ui.label(RichText::new(name).color(pal.text_dim));
                    }
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (label, kind) = match dialog {
                        Dialog::Rename { .. } => (t.save, ButtonKind::Primary),
                        Dialog::Delete { .. } => (t.delete, ButtonKind::Danger),
                    };
                    if widgets::button(ui, &pal, None, label, kind).clicked() {
                        apply = true;
                    }
                    if widgets::button(ui, &pal, None, t.cancel, ButtonKind::Secondary).clicked() {
                        close = true;
                    }
                });
            });
        if modal.should_close() {
            close = true;
        }
        if apply {
            match self.dialog.take() {
                Some(Dialog::Rename { index, text }) => {
                    let name = text.trim();
                    if !name.is_empty()
                        && let Some(p) = self.cfg.profiles.get_mut(index)
                    {
                        p.name = name.to_owned();
                    }
                }
                Some(Dialog::Delete { index }) if self.cfg.profiles.len() > 1 && index < self.cfg.profiles.len() => {
                    self.cfg.profiles.remove(index);
                    if self.cfg.active_profile >= index && self.cfg.active_profile > 0 {
                        self.cfg.active_profile -= 1;
                    }
                }
                _ => {}
            }
        } else if close {
            self.dialog = None;
        }
    }
}
