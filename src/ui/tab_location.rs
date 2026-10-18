//! "Location" tab: where to click (cursor, point, area, route) with a monitor
//! mini-map and on-screen pickers.

use egui::{Align, Align2, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

use super::icons;
use super::theme::{Palette, lerp, regular, semibold};
use super::widgets::{self, ButtonKind, captioned, card, num, row, segmented};
use crate::app::App;
use crate::platform::{self, MouseButton, PickKind, ScreenRect};
use crate::settings::{ActionKind, ClickKind, LocationMode, SeqOrder, Settings};

enum RouteOp {
    Up(usize),
    Down(usize),
    Remove(usize),
}

impl App {
    pub fn tab_location(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let ctx = ui.ctx().clone();
        let picking = self.picking;
        let running_index = self.controller.state().is_active().then(|| self.controller.stats.seq_index());
        let add_key = self.cfg.hotkeys.add_point.label();
        let mut pick: Option<PickKind> = None;
        let s = self.cfg.settings_mut();

        mode_tiles(ui, &pal, &mut s.location, t);

        let desc = match s.location {
            LocationMode::Cursor => t.loc_cursor_desc,
            LocationMode::Point => t.loc_point_desc,
            LocationMode::Area => t.loc_area_desc,
            LocationMode::Sequence => t.loc_route_desc,
        };
        widgets::hint(ui, &pal, desc);

        if s.action == ActionKind::Keyboard {
            note(ui, &pal, icons::INFO, t.loc_keyboard_note, pal.warning);
        }

        if s.location != LocationMode::Cursor {
            minimap(ui, &pal, s, running_index);
        }

        match s.location {
            LocationMode::Cursor => {}
            LocationMode::Point => {
                card(
                    ui,
                    &pal,
                    icons::CROSSHAIR,
                    t.loc_point,
                    |_| {},
                    |ui| {
                        ui.horizontal(|ui| {
                            captioned(ui, &pal, "X", |ui| num(ui, &mut s.point.0, -100_000..=100_000, 90.0, ""));
                            captioned(ui, &pal, "Y", |ui| num(ui, &mut s.point.1, -100_000..=100_000, 90.0, ""));
                            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                                let label = t.pick_on_screen;
                                if widgets::button(ui, &pal, Some(icons::TARGET), label, ButtonKind::Primary).clicked()
                                    && !picking
                                {
                                    pick = Some(PickKind::Point);
                                }
                            });
                        });
                    },
                );
            }
            LocationMode::Area => {
                card(
                    ui,
                    &pal,
                    icons::SELECTION,
                    t.loc_area,
                    |_| {},
                    |ui| {
                        let a = &mut s.area;
                        ui.horizontal(|ui| {
                            captioned(ui, &pal, "X", |ui| num(ui, &mut a.x, -100_000..=100_000, 80.0, ""));
                            captioned(ui, &pal, "Y", |ui| num(ui, &mut a.y, -100_000..=100_000, 80.0, ""));
                            captioned(ui, &pal, t.width, |ui| num(ui, &mut a.w, 1..=100_000, 80.0, ""));
                            captioned(ui, &pal, t.height, |ui| num(ui, &mut a.h, 1..=100_000, 80.0, ""));
                        });
                        if widgets::button(ui, &pal, Some(icons::SELECTION), t.select_on_screen, ButtonKind::Primary)
                            .clicked()
                            && !picking
                        {
                            pick = Some(PickKind::Area);
                        }
                    },
                );
            }
            LocationMode::Sequence => {
                let mut order = s.seq_order;
                let order_items = [(SeqOrder::InOrder, icons::LIST_NUMBERS), (SeqOrder::Random, icons::SHUFFLE)];
                let mut op: Option<RouteOp> = None;
                let mut clear = false;
                card(
                    ui,
                    &pal,
                    icons::PATH,
                    t.loc_route,
                    |ui| {
                        let r = segmented(ui, &pal, &mut order, &order_items, false);
                        r.on_hover_text(match order {
                            SeqOrder::InOrder => t.in_order,
                            SeqOrder::Random => t.random,
                        });
                    },
                    |ui| {
                        ui.horizontal(|ui| {
                            if widgets::button(ui, &pal, Some(icons::PLUS), t.add_on_screen, ButtonKind::Primary)
                                .clicked()
                                && !picking
                            {
                                pick = Some(PickKind::Points);
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                widgets::kbd(ui, &pal, &add_key, false);
                                ui.label(RichText::new(t.add_at_cursor).size(12.5).color(pal.text_dim));
                                ui.label(RichText::new(icons::CURSOR).size(14.0).color(pal.text_dim));
                            });
                        });
                        if s.sequence.is_empty() {
                            widgets::hint(ui, &pal, t.route_empty);
                        } else {
                            route_list(ui, &pal, &mut s.sequence, running_index, t, &mut op);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if widgets::button(ui, &pal, Some(icons::TRASH), t.clear, ButtonKind::Danger).clicked()
                                {
                                    clear = true;
                                }
                                let n = s.sequence.len() as u64;
                                ui.label(
                                    RichText::new(format!("{n} {}", t.points_word(n))).size(12.5).color(pal.text_dim),
                                );
                            });
                        }
                    },
                );
                s.seq_order = order;
                match op {
                    Some(RouteOp::Up(i)) if i > 0 => s.sequence.swap(i, i - 1),
                    Some(RouteOp::Down(i)) if i + 1 < s.sequence.len() => s.sequence.swap(i, i + 1),
                    Some(RouteOp::Remove(i)) if i < s.sequence.len() => {
                        s.sequence.remove(i);
                    }
                    _ => {}
                }
                if clear {
                    s.sequence.clear();
                }
            }
        }

        if s.location != LocationMode::Cursor {
            egui::Frame::new()
                .fill(pal.surface)
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(14)
                .inner_margin(egui::Margin::symmetric(16, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    row(ui, &pal, t.return_cursor, Some(t.return_cursor_hint), |ui| {
                        widgets::switch(ui, &pal, &mut s.return_cursor);
                    });
                });
        }

        if let Some(kind) = pick {
            self.start_pick(kind, &ctx);
        }
    }
}

fn mode_tiles(ui: &mut Ui, pal: &Palette, mode: &mut LocationMode, t: &crate::i18n::Tr) {
    let items = [
        (LocationMode::Cursor, icons::CURSOR, t.loc_cursor),
        (LocationMode::Point, icons::CROSSHAIR, t.loc_point),
        (LocationMode::Area, icons::SELECTION, t.loc_area),
        (LocationMode::Sequence, icons::PATH, t.loc_route),
    ];
    let gap = 8.0;
    let w = (ui.available_width() - gap * 3.0) / 4.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (value, icon, label) in items {
            let (rect, response) = ui.allocate_exact_size(vec2(w, 70.0), Sense::click());
            if response.clicked() {
                *mode = value;
            }
            let selected = *mode == value;
            let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
            let sel = ui.ctx().animate_bool_with_time(response.id.with("sel"), selected, 0.15);
            let p = ui.painter();
            let fill = lerp(lerp(pal.surface, pal.field, hover * 0.7), pal.accent_soft, sel);
            let stroke = Stroke::new(1.0 + sel * 0.5, lerp(pal.border, pal.accent, sel));
            p.rect(rect, 12, fill, stroke, StrokeKind::Inside);
            let icon_color = lerp(pal.text_dim, pal.accent, sel.max(hover * 0.4));
            p.text(rect.center() - vec2(0.0, 11.0), Align2::CENTER_CENTER, icon, regular(22.0), icon_color);
            let text_color = lerp(pal.text_dim, pal.text, sel.max(hover));
            p.text(rect.center() + vec2(0.0, 17.0), Align2::CENTER_CENTER, label, semibold(12.5), text_color);
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    });
}

fn note(ui: &mut Ui, pal: &Palette, icon: &str, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(if pal.dark { 0.12 } else { 0.1 }))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.35)))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            widgets::icon_text(ui, icon, color, text, pal.text, 12.5);
        });
}

/// Scaled drawing of all monitors with the current target(s) on top.
fn minimap(ui: &mut Ui, pal: &Palette, s: &Settings, running_index: Option<usize>) {
    let monitors = platform::monitors();
    let mut bounds = monitors[0];
    for m in &monitors[1..] {
        let x0 = bounds.x.min(m.x);
        let y0 = bounds.y.min(m.y);
        let x1 = (bounds.x + bounds.w).max(m.x + m.w);
        let y1 = (bounds.y + bounds.h).max(m.y + m.h);
        bounds = ScreenRect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }

    let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
    let p = ui.painter_at(outer);
    p.rect(outer, 14, pal.surface, Stroke::new(1.0, pal.border), StrokeKind::Inside);
    let area = outer.shrink(14.0);
    let scale = (area.width() / bounds.w as f32).min(area.height() / bounds.h as f32);
    let size = vec2(bounds.w as f32 * scale, bounds.h as f32 * scale);
    let origin = area.center() - size / 2.0;
    let map = |x: i32, y: i32| -> Pos2 { origin + vec2((x - bounds.x) as f32 * scale, (y - bounds.y) as f32 * scale) };

    for (i, m) in monitors.iter().enumerate() {
        let r = Rect::from_min_max(map(m.x, m.y), map(m.x + m.w, m.y + m.h)).shrink(1.5);
        p.rect(r, 6, pal.field, Stroke::new(1.0, pal.border), StrokeKind::Inside);
        p.text(r.left_top() + vec2(7.0, 5.0), Align2::LEFT_TOP, (i + 1).to_string(), semibold(10.5), pal.text_faint);
        let label = format!("{}×{}", m.w, m.h);
        p.text(r.right_bottom() - vec2(7.0, 5.0), Align2::RIGHT_BOTTOM, label, regular(10.0), pal.text_faint);
    }

    match s.location {
        LocationMode::Point => {
            let c = map(s.point.0, s.point.1);
            p.circle_filled(c, 11.0, pal.accent.gamma_multiply(0.18));
            p.circle_stroke(c, 7.0, Stroke::new(1.5, pal.accent));
            p.circle_filled(c, 3.0, pal.accent);
        }
        LocationMode::Area => {
            let a = s.area;
            let r = Rect::from_min_max(map(a.x, a.y), map(a.x + a.w, a.y + a.h));
            let r = if r.width() < 4.0 || r.height() < 4.0 {
                Rect::from_center_size(r.center(), r.size().max(vec2(4.0, 4.0)))
            } else {
                r
            };
            p.rect(r, 3, pal.accent.gamma_multiply(0.22), Stroke::new(1.5, pal.accent), StrokeKind::Middle);
        }
        LocationMode::Sequence => {
            let pts: Vec<Pos2> = s.sequence.iter().map(|q| map(q.x, q.y)).collect();
            if s.seq_order == SeqOrder::InOrder && pts.len() > 1 {
                p.line(pts.clone(), Stroke::new(1.5, pal.accent.gamma_multiply(0.55)));
            }
            for (i, c) in pts.iter().enumerate() {
                let current = running_index == Some(i);
                let r = if current { 9.0 } else { 7.5 };
                if current {
                    p.circle_filled(*c, 14.0, pal.accent.gamma_multiply(0.25));
                }
                p.circle_filled(*c, r, pal.accent);
                p.text(*c, Align2::CENTER_CENTER, (i + 1).to_string(), semibold(9.5), Color32::WHITE);
            }
        }
        LocationMode::Cursor => {}
    }
}

fn route_list(
    ui: &mut Ui,
    pal: &Palette,
    seq: &mut [crate::settings::SeqPoint],
    running_index: Option<usize>,
    t: &crate::i18n::Tr,
    op: &mut Option<RouteOp>,
) {
    let header = |ui: &mut Ui, text: &str, w: f32| {
        ui.add_sized(vec2(w, 14.0), egui::Label::new(RichText::new(text).size(11.0).color(pal.text_faint)));
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        header(ui, "#", 24.0);
        header(ui, "X", 62.0);
        header(ui, "Y", 62.0);
        header(ui, "", 30.0);
        header(ui, "", 34.0);
        header(ui, t.pause, 76.0);
    });
    let n = seq.len();
    for (i, point) in seq.iter_mut().enumerate() {
        let current = running_index == Some(i);
        let resp = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (badge, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
            let p = ui.painter();
            p.circle_filled(badge.center(), 11.0, if current { pal.accent } else { pal.field });
            let color = if current { Color32::WHITE } else { pal.text_dim };
            p.text(badge.center(), Align2::CENTER_CENTER, (i + 1).to_string(), semibold(11.0), color);

            num(ui, &mut point.x, -100_000..=100_000, 62.0, "");
            num(ui, &mut point.y, -100_000..=100_000, 62.0, "");

            let (icon, name) = match point.button {
                MouseButton::Left => (icons::MOUSE_LEFT_CLICK, t.btn_left),
                MouseButton::Middle => (icons::MOUSE_MIDDLE_CLICK, t.btn_middle),
                MouseButton::Right => (icons::MOUSE_RIGHT_CLICK, t.btn_right),
            };
            if small_toggle(ui, pal, icon, 30.0).on_hover_text(name).clicked() {
                point.button = match point.button {
                    MouseButton::Left => MouseButton::Right,
                    MouseButton::Right => MouseButton::Middle,
                    MouseButton::Middle => MouseButton::Left,
                };
            }
            let (kind, kind_name) = match point.click {
                ClickKind::Single => ("×1", t.single),
                ClickKind::Double => ("×2", t.double),
            };
            if small_toggle(ui, pal, kind, 34.0).on_hover_text(kind_name).clicked() {
                point.click = match point.click {
                    ClickKind::Single => ClickKind::Double,
                    ClickKind::Double => ClickKind::Single,
                };
            }
            num(ui, &mut point.pause_ms, 0..=3_600_000, 76.0, &format!(" {}", t.unit_ms)).on_hover_text(t.pause);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if widgets::icon_button_sized(ui, pal, icons::X, t.tip_remove, false, 24.0).clicked() {
                    *op = Some(RouteOp::Remove(i));
                }
                if i + 1 < n
                    && widgets::icon_button_sized(ui, pal, icons::ARROW_DOWN, t.tip_down, false, 24.0).clicked()
                {
                    *op = Some(RouteOp::Down(i));
                }
                if i > 0 && widgets::icon_button_sized(ui, pal, icons::ARROW_UP, t.tip_up, false, 24.0).clicked() {
                    *op = Some(RouteOp::Up(i));
                }
            });
        });
        if current {
            let r = resp.response.rect.expand2(vec2(4.0, 2.0));
            ui.painter().rect_stroke(r, 8, Stroke::new(1.0, pal.accent.gamma_multiply(0.6)), StrokeKind::Outside);
        }
    }
}

fn small_toggle(ui: &mut Ui, pal: &Palette, label: &str, w: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
    let p = ui.painter();
    p.rect(rect, 8, lerp(pal.field, pal.raised, hover), Stroke::new(1.0, pal.border), StrokeKind::Inside);
    p.text(rect.center(), Align2::CENTER_CENTER, label, regular(15.0), pal.text);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
