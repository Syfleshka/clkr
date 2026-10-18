//! "Click" tab: speed, action and stop condition.

use egui::{RichText, Ui, vec2};

use super::chrome::format_cps;
use super::icons;
use super::widgets::{self, captioned, card, chip, num, row, segmented};
use crate::app::App;
use crate::hotkeys::CaptureTarget;
use crate::platform::MouseButton;
use crate::settings::{ActionKind, ClickKind, Interval, LimitMode, MAX_CPS, MAX_HOLD_MS, MIN_CPS, RateMode};

const CPS_PRESETS: [f64; 7] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];

impl App {
    pub fn tab_click(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let capturing_key = self.capture == Some(CaptureTarget::ActionKey);
        let mut capture_key = false;

        // Speed
        let s = self.cfg.settings_mut();
        let mut rate_mode = s.rate_mode;
        card(
            ui,
            &pal,
            icons::GAUGE,
            t.speed,
            |ui| {
                segmented(
                    ui,
                    &pal,
                    &mut rate_mode,
                    &[(RateMode::Cps, t.rate_cps), (RateMode::Interval, t.rate_interval)],
                    false,
                );
            },
            |ui| {
                match s.rate_mode {
                    RateMode::Cps => {
                        ui.horizontal(|ui| {
                            let dv = egui::DragValue::new(&mut s.cps)
                                .range(MIN_CPS..=MAX_CPS)
                                .speed(0.1)
                                .max_decimals(2)
                                .suffix(format!(" /{}", t.unit_s));
                            ui.add_sized(vec2(96.0, 32.0), dv);
                            ui.spacing_mut().slider_width = (ui.available_width() - 8.0).max(60.0);
                            ui.add(egui::Slider::new(&mut s.cps, 0.5..=100.0).logarithmic(true).show_value(false));
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for preset in CPS_PRESETS {
                                if chip(ui, &pal, &format_cps(preset), (s.cps - preset).abs() < 1e-6).clicked() {
                                    s.cps = preset;
                                }
                            }
                        });
                    }
                    RateMode::Interval => {
                        interval_fields(ui, &pal, &mut s.interval, t);
                    }
                }
                let ms = s.interval_ms();
                let every = if ms >= 1000.0 {
                    format!("{} {}", format_cps(ms / 1000.0), t.unit_s)
                } else {
                    format!("{} {}", format_cps(ms), t.unit_ms)
                };
                ui.label(
                    RichText::new(format!(
                        "≈ {} {}  ·  {} {every}",
                        format_cps(s.effective_cps()),
                        t.stat_cps,
                        t.every
                    ))
                    .size(12.5)
                    .color(pal.text_dim),
                );
                widgets::divider(ui, &pal);
                row(ui, &pal, t.jitter, Some(t.jitter_hint), |ui| {
                    let mut on = s.jitter > 0.0;
                    if widgets::switch(ui, &pal, &mut on).changed() {
                        s.jitter = if on { 0.15 } else { 0.0 };
                    }
                    if on {
                        let mut pct = (s.jitter * 100.0).round() as u32;
                        if num(ui, &mut pct, 1..=50, 70.0, " %").changed() {
                            s.jitter = pct as f32 / 100.0;
                        }
                        ui.label(RichText::new("±").color(pal.text_dim));
                    }
                });
            },
        );
        s.rate_mode = rate_mode;

        // Action
        let mut action = s.action;
        let mouse_label = format!("{}  {}", icons::MOUSE, t.mouse);
        let key_label = format!("{}  {}", icons::KEYBOARD, t.keyboard);
        card(
            ui,
            &pal,
            icons::HAND_TAP,
            t.action,
            |ui| {
                segmented(
                    ui,
                    &pal,
                    &mut action,
                    &[(ActionKind::Mouse, mouse_label.as_str()), (ActionKind::Keyboard, key_label.as_str())],
                    false,
                );
            },
            |ui| {
                match s.action {
                    ActionKind::Mouse => {
                        row(ui, &pal, t.button, None, |ui| {
                            segmented(
                                ui,
                                &pal,
                                &mut s.button,
                                &[
                                    (MouseButton::Left, t.btn_left),
                                    (MouseButton::Middle, t.btn_middle),
                                    (MouseButton::Right, t.btn_right),
                                ],
                                false,
                            );
                        });
                        row(ui, &pal, t.click_type, None, |ui| {
                            segmented(
                                ui,
                                &pal,
                                &mut s.click,
                                &[(ClickKind::Single, t.single), (ClickKind::Double, t.double)],
                                false,
                            );
                        });
                    }
                    ActionKind::Keyboard => {
                        row(ui, &pal, t.key, Some(t.key_hint), |ui| {
                            if widgets::key_button(ui, &pal, &s.key.label(), capturing_key, t.press_key, 170.0)
                                .clicked()
                            {
                                capture_key = true;
                            }
                        });
                    }
                }
                row(ui, &pal, t.hold, Some(t.hold_hint), |ui| {
                    num(ui, &mut s.hold_ms, 0..=MAX_HOLD_MS, 96.0, &format!(" {}", t.unit_ms));
                });
            },
        );
        s.action = action;

        // Stop condition
        let mut limit = s.limit;
        let counts_cycles = s.counts_cycles();
        card(
            ui,
            &pal,
            icons::TIMER,
            t.stop_after,
            |ui| {
                segmented(
                    ui,
                    &pal,
                    &mut limit,
                    &[(LimitMode::Infinite, t.never), (LimitMode::Count, t.count), (LimitMode::Time, t.time)],
                    false,
                );
            },
            |ui| match s.limit {
                LimitMode::Infinite => {
                    widgets::icon_text(ui, icons::INFINITY, pal.text_dim, t.never_hint, pal.text_dim, 12.5);
                }
                LimitMode::Count => {
                    ui.horizontal(|ui| {
                        num(ui, &mut s.limit_count, 1..=1_000_000_000, 130.0, "");
                        let word =
                            if counts_cycles { t.cycles_word(s.limit_count) } else { t.clicks_word(s.limit_count) };
                        ui.label(RichText::new(word).color(pal.text_dim));
                    });
                }
                LimitMode::Time => {
                    let mut iv = Interval {
                        hours: (s.limit_secs / 3600) as u32,
                        minutes: ((s.limit_secs / 60) % 60) as u32,
                        seconds: (s.limit_secs % 60) as u32,
                        millis: 0,
                    };
                    ui.horizontal(|ui| {
                        captioned(ui, &pal, t.unit_h, |ui| num(ui, &mut iv.hours, 0..=999, 74.0, ""));
                        captioned(ui, &pal, t.unit_min, |ui| num(ui, &mut iv.minutes, 0..=59, 74.0, ""));
                        captioned(ui, &pal, t.unit_s, |ui| num(ui, &mut iv.seconds, 0..=59, 74.0, ""));
                    });
                    s.limit_secs = (iv.hours as u64 * 3600 + iv.minutes as u64 * 60 + iv.seconds as u64).max(1);
                }
            },
        );
        s.limit = limit;

        if capture_key {
            self.begin_capture(CaptureTarget::ActionKey);
        }
    }
}

fn interval_fields(ui: &mut Ui, pal: &super::theme::Palette, iv: &mut Interval, t: &crate::i18n::Tr) {
    ui.horizontal(|ui| {
        captioned(ui, pal, t.unit_h, |ui| num(ui, &mut iv.hours, 0..=99, 74.0, ""));
        captioned(ui, pal, t.unit_min, |ui| num(ui, &mut iv.minutes, 0..=59, 74.0, ""));
        captioned(ui, pal, t.unit_s, |ui| num(ui, &mut iv.seconds, 0..=59, 74.0, ""));
        captioned(ui, pal, t.unit_ms, |ui| num(ui, &mut iv.millis, 0..=999, 84.0, ""));
    });
    if iv.total_ms() == 0 {
        iv.millis = 1;
    }
}
