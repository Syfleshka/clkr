//! Window chrome and the always-visible parts: title bar, status card with the
//! start button, and the compact view.

use egui::epaint::RectShape;
use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, Margin, PointerButton, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, ViewportCommand, pos2, vec2,
};

use super::icons;
use super::theme::{lerp, regular, semibold};
use super::widgets::{self, icon_button};
use crate::app::{App, Dialog};
use crate::engine::RunState;
use crate::i18n::Lang;
use crate::settings::{ActionKind, LimitMode, LocationMode, Profile, Settings, ThemeChoice};

impl App {
    pub fn title_bar(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let rect = ui.max_rect();
        let drag = ui.interact(rect, Id::unique("title-drag"), Sense::click_and_drag());
        if drag.drag_started_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let inner = rect.shrink2(vec2(12.0, 0.0));
        ui.scope_builder(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            self.logo(ui);
            ui.label(RichText::new("clkr").font(semibold(16.0)).color(pal.text));
            if !self.cfg.compact {
                ui.add_space(4.0);
                self.profile_picker(ui);
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if icon_button(ui, &pal, icons::X, t.tip_close, false).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                if !self.cfg.compact && icon_button(ui, &pal, icons::MINUS, t.tip_minimize, false).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                let (icon, tip) = if self.cfg.compact {
                    (icons::CORNERS_OUT, t.tip_expand)
                } else {
                    (icons::CORNERS_IN, t.tip_compact)
                };
                if icon_button(ui, &pal, icon, tip, false).clicked() {
                    self.cfg.compact = !self.cfg.compact;
                }
                let on_top = self.cfg.always_on_top;
                let (icon, tip) =
                    if on_top { (icons::PUSH_PIN, t.tip_unpin) } else { (icons::PUSH_PIN_SLASH, t.tip_pin) };
                if icon_button(ui, &pal, icon, tip, on_top).clicked() {
                    self.cfg.always_on_top = !on_top;
                }
                if !self.cfg.compact {
                    let (icon, tip, next) = match self.cfg.theme {
                        ThemeChoice::Dark => (icons::SUN, t.tip_theme_light, ThemeChoice::Light),
                        ThemeChoice::Light => (icons::MOON, t.tip_theme_dark, ThemeChoice::Dark),
                    };
                    if icon_button(ui, &pal, icon, tip, false).clicked() {
                        self.cfg.theme = next;
                    }
                    let (code, next) = match self.cfg.language {
                        Lang::En => ("EN", Lang::Ru),
                        Lang::Ru => ("RU", Lang::En),
                    };
                    if lang_button(ui, &pal, code, t.tip_language).clicked() {
                        self.cfg.language = next;
                    }
                }
            });
        });
    }

    fn logo(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
        let p = ui.painter();
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        p.add(RectShape::filled(rect, 8, Color32::WHITE).with_texture(self.grad_idle.id(), uv));
        p.text(
            rect.center() + vec2(0.5, 0.5),
            Align2::CENTER_CENTER,
            icons::CURSOR_CLICK,
            regular(16.0),
            Color32::WHITE,
        );
    }

    fn profile_picker(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let name = self.cfg.profiles[self.cfg.active_profile].name.clone();
        // Long names are cut with an ellipsis so the title bar buttons stay visible.
        let mut job = egui::text::LayoutJob::single_section(
            name.clone(),
            egui::TextFormat { font_id: regular(13.0), color: pal.text, ..Default::default() },
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(104.0);
        let galley = ui.painter().layout_job(job);
        let width = galley.size().x + 50.0;
        let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
        {
            let p = ui.painter();
            p.rect(
                rect,
                8,
                lerp(pal.bg, pal.field, 0.6 + 0.4 * hover),
                Stroke::new(1.0, pal.border),
                StrokeKind::Inside,
            );
            p.text(
                pos2(rect.left() + 13.0, rect.center().y),
                Align2::CENTER_CENTER,
                icons::STACK,
                regular(14.0),
                pal.accent,
            );
            p.galley(pos2(rect.left() + 25.0, rect.center().y - galley.size().y / 2.0), galley, pal.text);
            let caret = pos2(rect.right() - 12.0, rect.center().y);
            p.text(caret, Align2::CENTER_CENTER, icons::CARET_DOWN, regular(12.0), pal.text_dim);
        }
        let response = response.on_hover_text(t.profiles).on_hover_cursor(egui::CursorIcon::PointingHand);

        let mut action: Option<ProfileAction> = None;
        egui::Popup::menu(&response).gap(6.0).width(230.0).show(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(t.profiles).size(11.5).color(pal.text_dim));
            for (i, p) in self.cfg.profiles.iter().enumerate() {
                let active = i == self.cfg.active_profile;
                let icon = if active { icons::CHECK } else { "" };
                if menu_item(ui, &pal, icon, &p.name, active).clicked() {
                    action = Some(ProfileAction::Select(i));
                }
            }
            widgets::divider(ui, &pal);
            if menu_item(ui, &pal, icons::PLUS, t.profile_new, false).clicked() {
                action = Some(ProfileAction::New);
            }
            if menu_item(ui, &pal, icons::COPY, t.profile_duplicate, false).clicked() {
                action = Some(ProfileAction::Duplicate);
            }
            if menu_item(ui, &pal, icons::PENCIL_SIMPLE, t.profile_rename, false).clicked() {
                action = Some(ProfileAction::Rename);
            }
            if self.cfg.profiles.len() > 1 && menu_item(ui, &pal, icons::TRASH, t.profile_delete, false).clicked() {
                action = Some(ProfileAction::Delete);
            }
        });

        match action {
            Some(ProfileAction::Select(i)) => self.cfg.active_profile = i,
            Some(ProfileAction::New) => {
                let name = format!("{} {}", t.profile_default_name, self.cfg.profiles.len() + 1);
                self.cfg.profiles.push(Profile { name, settings: Settings::default() });
                self.cfg.active_profile = self.cfg.profiles.len() - 1;
            }
            Some(ProfileAction::Duplicate) => {
                let mut copy = self.cfg.profiles[self.cfg.active_profile].clone();
                copy.name = format!("{} ({})", copy.name, t.profile_copy_suffix);
                self.cfg.profiles.insert(self.cfg.active_profile + 1, copy);
                self.cfg.active_profile += 1;
            }
            Some(ProfileAction::Rename) => {
                self.dialog = Some(Dialog::Rename { index: self.cfg.active_profile, text: name });
            }
            Some(ProfileAction::Delete) => self.dialog = Some(Dialog::Delete { index: self.cfg.active_profile }),
            None => {}
        }
    }

    /// Status card: state, counters, live rate graph and the big start button.
    pub fn hero(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let stats = self.controller.stats.clone();
        let state = stats.state();
        let s = self.cfg.settings().clone();
        let problem = s.problem(&self.cfg.hotkeys);

        egui::Frame::new()
            .fill(pal.surface)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(16)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 10.0;

                // Status line + summary of the current setup (dropped if it doesn't fit).
                let (text, color, pulse) = self.status(state);
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
                widgets::status_dot(ui, pos2(rect.left() + 5.0, rect.center().y), color, pulse);
                let status = ui.painter().layout_no_wrap(text, semibold(13.5), color);
                let summary = ui.painter().layout_no_wrap(self.summary(&s), regular(12.5), pal.text_dim);
                let status_w = status.size().x;
                ui.painter().galley(pos2(rect.left() + 18.0, rect.center().y - status.size().y / 2.0), status, color);
                if 18.0 + status_w + 24.0 + summary.size().x <= rect.width() {
                    let at = pos2(rect.right() - summary.size().x, rect.center().y - summary.size().y / 2.0);
                    ui.painter().galley(at, summary, pal.text_dim);
                }

                // Counters.
                let clicks = stats.clicks();
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                let p = ui.painter();
                let big = p.layout_no_wrap(group_digits(clicks), semibold(38.0), pal.text);
                let big_rect = Align2::LEFT_CENTER.anchor_size(rect.left_center(), big.size());
                p.galley(big_rect.min, big, pal.text);
                let unit_pos = pos2(big_rect.right() + 8.0, big_rect.bottom() - 9.0);
                p.text(unit_pos, Align2::LEFT_BOTTOM, t.clicks_word(clicks), regular(13.0), pal.text_dim);
                let mut x = rect.right();
                let cps = format!("{:.1}", self.meter.cps);
                for (value, label) in [(cps, t.stat_cps), (format_duration(stats.elapsed()), t.stat_time)] {
                    let v = p.layout_no_wrap(value, semibold(17.0), pal.text);
                    let l = p.layout_no_wrap(label.to_owned(), regular(11.5), pal.text_dim);
                    let w = v.size().x.max(l.size().x).max(58.0);
                    p.galley(pos2(x - v.size().x, rect.center().y - 18.0), v, pal.text);
                    p.galley(pos2(x - l.size().x, rect.center().y + 3.0), l, pal.text_dim);
                    x -= w + 20.0;
                }

                if let Some((done, total, unit)) = self.progress(&s, &stats) {
                    let frac = if total > 0.0 { (done / total).clamp(0.0, 1.0) } else { 0.0 };
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
                    let bar =
                        Rect::from_min_size(pos2(rect.left(), rect.center().y - 2.5), vec2(rect.width() * 0.72, 5.0));
                    let p = ui.painter();
                    p.rect_filled(bar, 3, pal.field);
                    if frac > 0.0 {
                        let fill = Rect::from_min_size(bar.min, vec2(bar.width() * frac as f32, bar.height()));
                        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(frac as f32, 1.0));
                        p.add(RectShape::filled(fill, 3, Color32::WHITE).with_texture(self.grad_idle.id(), uv));
                    }
                    p.text(rect.right_center(), Align2::RIGHT_CENTER, unit, regular(12.0), pal.text_dim);
                }

                self.sparkline(ui, state.is_active());

                if let Some(problem) = problem
                    && !state.is_active()
                {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icons::WARNING).size(15.0).color(pal.warning));
                        ui.label(RichText::new(self.problem_text(problem)).size(13.0).color(pal.warning));
                    });
                }

                let enabled = state.is_active() || problem.is_none();
                if self.start_button(ui, state, enabled, 54.0).clicked() && enabled {
                    self.controller.toggle();
                }
            });
    }

    fn status(&self, state: RunState) -> (String, Color32, bool) {
        let pal = &self.pal;
        let t = self.t();
        let stats = &self.controller.stats;
        match state {
            RunState::Idle => (t.status_ready.to_owned(), pal.text_dim, false),
            RunState::Starting => (t.status_starting.to_owned(), pal.warning, true),
            RunState::Countdown => {
                let left = stats.countdown_left().map_or(0, |d| d.as_secs_f32().ceil() as u32);
                (format!("{} {left}…", t.status_countdown), pal.warning, true)
            }
            RunState::Running if stats.blocked() => {
                let text = if self.cfg.settings().action == ActionKind::Keyboard {
                    t.status_paused_key
                } else {
                    t.status_paused_mouse
                };
                (text.to_owned(), pal.warning, false)
            }
            RunState::Running => (t.status_running.to_owned(), pal.success, true),
        }
    }

    fn summary(&self, s: &Settings) -> String {
        let t = self.t();
        let rate = format_cps(s.effective_cps());
        let what = match s.action {
            ActionKind::Keyboard => format!("{}  {}", icons::KEYBOARD, s.key.label()),
            ActionKind::Mouse => {
                let b = match s.button {
                    crate::platform::MouseButton::Left => t.btn_left,
                    crate::platform::MouseButton::Middle => t.btn_middle,
                    crate::platform::MouseButton::Right => t.btn_right,
                };
                let loc = match s.location {
                    LocationMode::Cursor => t.loc_cursor,
                    LocationMode::Point => t.loc_point,
                    LocationMode::Area => t.loc_area,
                    LocationMode::Sequence => t.loc_route,
                };
                format!("{}  {b} · {loc}", icons::MOUSE)
            }
        };
        format!("{}  {rate}/{}   {what}", icons::LIGHTNING, t.unit_s)
    }

    fn progress(&self, s: &Settings, stats: &crate::engine::Stats) -> Option<(f64, f64, String)> {
        let t = self.t();
        match s.limit {
            LimitMode::Infinite => None,
            LimitMode::Count => {
                let total = s.limit_count.max(1);
                let (done, word) = if s.counts_cycles() {
                    (stats.cycles(), t.cycles_word(total))
                } else {
                    (stats.clicks(), t.clicks_word(total))
                };
                Some((
                    done as f64,
                    total as f64,
                    format!("{} {} {} {word}", group_digits(done), t.of, group_digits(total)),
                ))
            }
            LimitMode::Time => {
                let total = s.limit_secs.max(1) as f64;
                let done = stats.elapsed().as_secs_f64().min(total);
                let label = format!(
                    "{} {} {}",
                    format_duration(stats.elapsed()),
                    t.of,
                    format_duration(std::time::Duration::from_secs(s.limit_secs.max(1)))
                );
                Some((done, total, label))
            }
        }
    }

    fn sparkline(&self, ui: &mut Ui, active: bool) {
        let pal = self.pal;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
        let p = ui.painter_at(rect);
        let hist = &self.meter.history;
        let n = crate::app::CpsMeter::HISTORY;
        p.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, pal.border));
        if hist.len() < 2 {
            return;
        }
        let max = hist.iter().copied().fold(1.0f32, f32::max) * 1.15;
        let step = rect.width() / (n - 1) as f32;
        let x0 = rect.right() - step * (hist.len() - 1) as f32;
        let pts: Vec<Pos2> = hist
            .iter()
            .enumerate()
            .map(|(i, v)| pos2(x0 + step * i as f32, rect.bottom() - 2.0 - (v / max) * (rect.height() - 6.0)))
            .collect();

        let color = if active { pal.accent } else { pal.text_faint };
        let mut mesh = egui::Mesh::default();
        let top = color.gamma_multiply(if pal.dark { 0.35 } else { 0.22 });
        for w in pts.windows(2) {
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(w[0], top);
            mesh.colored_vertex(w[1], top);
            mesh.colored_vertex(pos2(w[1].x, rect.bottom()), Color32::TRANSPARENT);
            mesh.colored_vertex(pos2(w[0].x, rect.bottom()), Color32::TRANSPARENT);
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        p.add(mesh);
        p.line(pts, Stroke::new(1.8, color));
    }

    /// The big gradient start/stop button.
    pub fn start_button(&self, ui: &mut Ui, state: RunState, enabled: bool, height: f32) -> egui::Response {
        let pal = self.pal;
        let t = self.t();
        let active = state.is_active();
        let size = vec2(ui.available_width(), height);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered() && enabled, 0.12);
        let pressed = response.is_pointer_button_down_on() && enabled;
        let r = if pressed { rect.shrink(1.0) } else { rect };
        let radius = CornerRadius::same((height * 0.26) as u8);
        let p = ui.painter();
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

        if enabled {
            let tex = if active { self.grad_run.id() } else { self.grad_idle.id() };
            let glow_color = if active { pal.danger } else { pal.accent };
            let pulse = if active {
                let time = ui.input(|i| i.time) as f32;
                0.5 + 0.5 * (time * 3.0).sin()
            } else {
                hover
            };
            let glow = glow_color.gamma_multiply(if pal.dark { 0.25 + 0.25 * pulse } else { 0.18 + 0.18 * pulse });
            p.add(
                RectShape::filled(r.translate(vec2(0.0, 4.0)).shrink2(vec2(10.0, 4.0)), radius, glow)
                    .with_blur_width(18.0),
            );
            p.add(RectShape::filled(r, radius, Color32::WHITE).with_texture(tex, uv));
            if hover > 0.0 {
                p.rect_filled(r, radius, Color32::from_white_alpha((18.0 * hover) as u8));
            }
        } else {
            p.rect(r, radius, pal.field, Stroke::new(1.0, pal.border), StrokeKind::Inside);
        }

        let fg = if enabled { Color32::WHITE } else { pal.text_faint };
        let label = if active { t.stop } else { t.start };
        let text = ui.painter().layout_no_wrap(label.to_owned(), semibold(height * 0.32), fg);
        let hotkey = self.cfg.hotkeys.toggle.label();
        let kbd = ui.painter().layout_no_wrap(hotkey, semibold(11.5), fg);
        let icon_w = height * 0.26;
        let total = icon_w + 12.0 + text.size().x + 12.0 + kbd.size().x + 12.0;
        let mut x = r.center().x - total / 2.0;
        let cy = r.center().y;
        let p = ui.painter();
        if active {
            let sq = Rect::from_center_size(pos2(x + icon_w / 2.0, cy), egui::Vec2::splat(icon_w * 0.85));
            p.rect_filled(sq, 3, fg);
        } else {
            let h = icon_w;
            let pts = vec![pos2(x + 1.0, cy - h / 2.0), pos2(x + 1.0 + h * 0.9, cy), pos2(x + 1.0, cy + h / 2.0)];
            p.add(egui::Shape::convex_polygon(pts, fg, Stroke::NONE));
        }
        x += icon_w + 12.0;
        p.galley(pos2(x, cy - text.size().y / 2.0), text.clone(), fg);
        x += text.size().x + 12.0;
        let kbd_rect = Rect::from_min_size(pos2(x, cy - 10.0), vec2(kbd.size().x + 12.0, 20.0));
        let (kfill, kstroke) = if enabled {
            (Color32::from_white_alpha(36), Color32::from_white_alpha(70))
        } else {
            (pal.surface, pal.border)
        };
        p.rect(kbd_rect, 5, kfill, Stroke::new(1.0, kstroke), StrokeKind::Inside);
        p.galley(kbd_rect.center() - kbd.size() / 2.0, kbd, fg);

        if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
    }

    pub fn compact_view(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let stats = self.controller.stats.clone();
        let state = stats.state();
        let problem = self.cfg.settings().problem(&self.cfg.hotkeys);
        let clicks = stats.clicks();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.horizontal(|ui| {
                    let (text, color, pulse) = self.status(state);
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
                    widgets::status_dot(ui, dot.center(), color, pulse);
                    ui.label(RichText::new(text).size(12.0).color(color));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(group_digits(clicks)).font(semibold(24.0)));
                    ui.label(
                        RichText::new(format!("{}  ·  {:.1}/{}", t.clicks_word(clicks), self.meter.cps, t.unit_s))
                            .size(12.0)
                            .color(pal.text_dim),
                    );
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.set_max_width(170.0);
                let enabled = state.is_active() || problem.is_none();
                if self.start_button(ui, state, enabled, 44.0).clicked() && enabled {
                    self.controller.toggle();
                }
            });
        });
    }
}

enum ProfileAction {
    Select(usize),
    New,
    Duplicate,
    Rename,
    Delete,
}

fn menu_item(ui: &mut Ui, pal: &super::theme::Palette, icon: &str, text: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.08);
    let p = ui.painter();
    if hover > 0.0 {
        p.rect_filled(rect, 7, pal.field.gamma_multiply(hover.max(0.0)));
    }
    let color = if active { pal.accent } else { pal.text };
    p.text(pos2(rect.left() + 16.0, rect.center().y), Align2::CENTER_CENTER, icon, regular(15.0), pal.accent);
    let clip = Rect::from_min_max(pos2(rect.left() + 30.0, rect.top()), pos2(rect.right() - 6.0, rect.bottom()));
    p.with_clip_rect(clip).text(
        pos2(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        regular(13.5),
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn lang_button(ui: &mut Ui, pal: &super::theme::Palette, code: &str, tip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, 30.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
    let p = ui.painter();
    if hover > 0.0 {
        p.rect_filled(rect, 8, pal.field.gamma_multiply(hover));
    }
    let chip = Rect::from_center_size(rect.center(), vec2(26.0, 18.0));
    p.rect_stroke(chip, 5, Stroke::new(1.2, lerp(pal.text_dim, pal.text, hover)), StrokeKind::Inside);
    p.text(chip.center(), Align2::CENTER_CENTER, code, semibold(10.5), lerp(pal.text_dim, pal.text, hover));
    response.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// `1234567` → `1 234 567`.
pub fn group_digits(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202F}');
        }
        out.push(ch);
    }
    out
}

pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}.{}", d.subsec_millis() / 100) }
}

pub fn format_cps(cps: f64) -> String {
    if cps >= 100.0 {
        format!("{cps:.0}")
    } else if cps >= 10.0 {
        format!("{cps:.1}").trim_end_matches(".0").to_owned()
    } else {
        let s = format!("{cps:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1_234_567), "1\u{202F}234\u{202F}567");
        assert_eq!(format_cps(10.0), "10");
        assert_eq!(format_cps(12.5), "12.5");
        assert_eq!(format_cps(0.5), "0.5");
        assert_eq!(format_cps(250.0), "250");
        assert_eq!(format_duration(std::time::Duration::from_millis(65_400)), "01:05.4");
        assert_eq!(format_duration(std::time::Duration::from_secs(3_725)), "1:02:05");
    }
}
