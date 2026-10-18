//! Custom widgets: cards, segmented controls, switches, buttons, keycaps.

use egui::text::LayoutJob;
use egui::{
    Align, Align2, Color32, Label, Layout, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextFormat, Ui,
    Vec2, vec2,
};

use super::theme::{Palette, lerp, regular, semibold};

/// A rounded panel with an icon + title header and optional header controls.
pub fn card<R>(
    ui: &mut Ui,
    pal: &Palette,
    icon: &str,
    title: &str,
    header_right: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(14)
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let w = ui.available_width();
            ui.allocate_ui_with_layout(vec2(w, 30.0), Layout::left_to_right(Align::Center), |ui| {
                ui.set_min_size(vec2(w, 30.0));
                ui.label(RichText::new(icon).size(17.0).color(pal.accent));
                ui.add_space(-2.0);
                ui.label(RichText::new(title).font(semibold(15.0)).color(pal.text));
                ui.with_layout(Layout::right_to_left(Align::Center), header_right);
            });
            ui.add_space(4.0);
            body(ui)
        })
        .inner
}

/// A settings row: label (and optional hint) on the left, controls on the right.
pub fn row(ui: &mut Ui, pal: &Palette, label: &str, hint: Option<&str>, right: impl FnOnce(&mut Ui)) {
    let h = if hint.is_some() { 44.0 } else { 34.0 };
    let w = ui.available_width();
    ui.allocate_ui_with_layout(vec2(w, h), Layout::right_to_left(Align::Center), |ui| {
        ui.set_min_size(vec2(w, h));
        right(ui);
        ui.add_space(10.0);
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add(Label::new(label_job(pal, label, hint)).wrap());
        });
    });
}

fn label_job(pal: &Palette, label: &str, hint: Option<&str>) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(label, 0.0, TextFormat { font_id: regular(14.0), color: pal.text, ..Default::default() });
    if let Some(hint) = hint {
        job.append("\n", 0.0, TextFormat { font_id: regular(6.0), color: pal.text, ..Default::default() });
        job.append(hint, 0.0, TextFormat { font_id: regular(12.0), color: pal.text_dim, ..Default::default() });
    }
    job
}

/// Dim helper text.
pub fn hint(ui: &mut Ui, pal: &Palette, text: &str) {
    ui.add(Label::new(RichText::new(text).size(12.5).color(pal.text_dim)).wrap());
}

/// Icon followed by wrapping text, as one label so the text wraps correctly.
pub fn icon_text(ui: &mut Ui, icon: &str, icon_color: Color32, text: &str, color: Color32, size: f32) {
    let mut job = LayoutJob::default();
    job.append(icon, 0.0, TextFormat { font_id: regular(size + 2.0), color: icon_color, ..Default::default() });
    job.append(text, 7.0, TextFormat { font_id: regular(size), color, ..Default::default() });
    ui.add(Label::new(job).wrap());
}

/// Thin separator line in the card colour scheme.
pub fn divider(ui: &mut Ui, pal: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, pal.border));
}

/// iOS-style segmented control with an animated thumb.
///
/// With `fill` the control takes the whole available width, otherwise it is
/// as wide as its longest label requires.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    pal: &Palette,
    value: &mut T,
    items: &[(T, &str)],
    fill: bool,
) -> Response {
    let n = items.len().max(1);
    let font = regular(13.5);
    let galleys: Vec<_> = items
        .iter()
        .map(|(_, label)| ui.painter().layout_no_wrap((*label).to_owned(), font.clone(), pal.text))
        .collect();
    let pad = 3.0;
    let seg_w = if fill {
        (ui.available_width() - pad * 2.0) / n as f32
    } else {
        galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 24.0
    };
    let size = vec2(seg_w * n as f32 + pad * 2.0, 32.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::hover());
    let id = response.id;

    let selected = items.iter().position(|(v, _)| v == value).unwrap_or(0);
    let mut clicked_any = false;
    for (i, (v, _)) in items.iter().enumerate() {
        let seg = Rect::from_min_size(rect.min + vec2(pad + seg_w * i as f32, pad), vec2(seg_w, size.y - pad * 2.0));
        let r = ui.interact(seg, id.with(i), Sense::click());
        if r.clicked() && i != selected {
            *value = *v;
            clicked_any = true;
        }
        response = response.union(r);
    }
    if clicked_any {
        response.mark_changed();
    }

    let target = items.iter().position(|(v, _)| v == value).unwrap_or(0) as f32;
    let pos = ui.ctx().animate_value_with_time(id.with("thumb"), target, 0.16);
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 10, pal.field);
        let thumb = Rect::from_min_size(rect.min + vec2(pad + seg_w * pos, pad), vec2(seg_w, size.y - pad * 2.0));
        if !pal.dark {
            p.add(
                egui::epaint::RectShape::filled(thumb.translate(vec2(0.0, 1.0)), 8, Color32::from_black_alpha(28))
                    .with_blur_width(4.0),
            );
        }
        p.rect_filled(thumb, 8, pal.raised);
        for (i, g) in galleys.into_iter().enumerate() {
            let seg =
                Rect::from_min_size(rect.min + vec2(pad + seg_w * i as f32, pad), vec2(seg_w, size.y - pad * 2.0));
            let closeness = 1.0 - (pos - i as f32).abs().min(1.0);
            let hovered = ui.rect_contains_pointer(seg);
            let base = if hovered { pal.text } else { pal.text_dim };
            let color = lerp(base, pal.text, closeness);
            let at = seg.center() - g.size() / 2.0;
            p.galley_with_override_text_color(at, g, color);
        }
    }
    response
}

/// Animated on/off switch.
pub fn switch(ui: &mut Ui, pal: &Palette, on: &mut bool) -> Response {
    let size = vec2(40.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);
    if ui.is_rect_visible(rect) {
        let off = if pal.dark { pal.raised } else { Color32::from_rgb(214, 218, 230) };
        let track = lerp(off, pal.accent, t);
        let p = ui.painter();
        p.rect_filled(rect, 11, track);
        let knob_x = egui::lerp(rect.left() + 11.0..=rect.right() - 11.0, t);
        p.circle_filled(egui::pos2(knob_x, rect.center().y + 0.5), 8.5, Color32::from_black_alpha(40));
        p.circle_filled(egui::pos2(knob_x, rect.center().y), 8.0, Color32::WHITE);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Square icon button for toolbars / title bar.
pub fn icon_button(ui: &mut Ui, pal: &Palette, icon: &str, tip: &str, active: bool) -> Response {
    icon_button_sized(ui, pal, icon, tip, active, 30.0)
}

pub fn icon_button_sized(ui: &mut Ui, pal: &Palette, icon: &str, tip: &str, active: bool, side: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    if ui.is_rect_visible(rect) {
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
        let p = ui.painter();
        if hover > 0.0 {
            p.rect_filled(rect, 8, pal.field.gamma_multiply(hover));
        }
        let color = if active { pal.accent } else { lerp(pal.text_dim, pal.text, hover) };
        p.text(rect.center(), Align2::CENTER_CENTER, icon, regular(side * 0.55), color);
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tip.is_empty() { response } else { response.on_hover_text(tip) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

/// Rounded text button with an optional leading icon.
pub fn button(ui: &mut Ui, pal: &Palette, icon: Option<&str>, text: &str, kind: ButtonKind) -> Response {
    let font = regular(13.5);
    let label = match icon {
        Some(icon) => format!("{icon}  {text}"),
        None => text.to_owned(),
    };
    let galley = ui.painter().layout_no_wrap(label, font, pal.text);
    let size = vec2(galley.size().x + 26.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
        let pressed = response.is_pointer_button_down_on();
        let p = ui.painter();
        let (fill, stroke, text) = match kind {
            ButtonKind::Primary => {
                let fill = lerp(pal.accent, lerp(pal.accent, Color32::WHITE, 0.15), hover);
                (fill, Stroke::NONE, pal.on_accent)
            }
            ButtonKind::Secondary => (lerp(pal.field, pal.raised, hover), Stroke::new(1.0, pal.border), pal.text),
            ButtonKind::Danger => {
                (lerp(pal.field, pal.danger.gamma_multiply(0.18), hover), Stroke::new(1.0, pal.border), pal.danger)
            }
        };
        let r = if pressed { rect.shrink(0.5) } else { rect };
        p.rect(r, 9, fill, stroke, StrokeKind::Inside);
        p.galley_with_override_text_color(r.center() - galley.size() / 2.0, galley, text);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Small selectable pill, e.g. speed presets.
pub fn chip(ui: &mut Ui, pal: &Palette, text: &str, selected: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), regular(12.5), pal.text);
    let size = vec2((galley.size().x + 18.0).max(34.0), 26.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
        let p = ui.painter();
        let (fill, stroke, color) = if selected {
            (pal.accent_soft, Stroke::new(1.0, pal.accent.gamma_multiply(0.7)), pal.accent)
        } else {
            (lerp(pal.field, pal.raised, hover), Stroke::NONE, lerp(pal.text_dim, pal.text, hover))
        };
        p.rect(rect, 13, fill, stroke, StrokeKind::Inside);
        p.galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, color);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Keycap-looking label.
pub fn kbd(ui: &mut Ui, pal: &Palette, text: &str, on_accent: bool) -> Response {
    let color = if on_accent { Color32::WHITE } else { pal.text_dim };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), semibold(11.5), color);
    let size = vec2(galley.size().x + 12.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    let (fill, stroke) = if on_accent {
        (Color32::from_white_alpha(38), Stroke::new(1.0, Color32::from_white_alpha(60)))
    } else {
        (pal.field, Stroke::new(1.0, pal.border))
    };
    p.rect(rect, 5, fill, stroke, StrokeKind::Inside);
    p.galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

/// Button that shows a key combination and starts capturing a new one on click.
pub fn key_button(ui: &mut Ui, pal: &Palette, label: &str, capturing: bool, prompt: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
        let p = ui.painter();
        if capturing {
            let t = ui.input(|i| i.time) as f32;
            let pulse = 0.55 + 0.45 * (t * 5.0).sin().abs();
            p.rect(rect, 9, pal.accent_soft, Stroke::new(1.5, pal.accent.gamma_multiply(pulse)), StrokeKind::Inside);
            p.text(rect.center(), Align2::CENTER_CENTER, prompt, regular(13.0), pal.accent);
            ui.ctx().request_repaint();
        } else {
            p.rect(rect, 9, lerp(pal.field, pal.raised, hover), Stroke::new(1.0, pal.border), StrokeKind::Inside);
            p.text(rect.center(), Align2::CENTER_CENTER, label, semibold(13.5), pal.text);
        }
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Fixed-width numeric field.
pub fn num<N: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    width: f32,
    suffix: &str,
) -> Response {
    let dv = egui::DragValue::new(value).range(range).speed(0.2).suffix(suffix.to_owned());
    ui.add_sized(vec2(width, 30.0), dv)
}

/// Field with a caption above it (used for X / Y / W / H and h / min / s / ms).
pub fn captioned<R>(ui: &mut Ui, pal: &Palette, caption: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.label(RichText::new(caption).size(11.5).color(pal.text_dim));
        add(ui)
    })
    .inner
}

/// Little coloured dot; pulses when `pulse` is set.
pub fn status_dot(ui: &Ui, center: egui::Pos2, color: Color32, pulse: bool) {
    let p = ui.painter();
    if pulse {
        let t = ui.input(|i| i.time) as f32;
        let phase = (t * 1.6).fract();
        p.circle_filled(center, 4.0 + phase * 7.0, color.gamma_multiply(0.45 * (1.0 - phase)));
    }
    p.circle_filled(center, 4.0, color);
}
