//! "Settings" tab: hotkeys, start & safety, interface and statistics.

use egui::{RichText, Ui};

use super::chrome::group_digits;
use super::icons;
use super::widgets::{self, ButtonKind, card, num, row, segmented};
use crate::app::App;
use crate::hotkeys::CaptureTarget;
use crate::i18n::Lang;
use crate::settings::{HotkeyMode, ThemeChoice};

impl App {
    pub fn tab_settings(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        let t = self.t();
        let capture = self.capture;
        let mut begin: Option<CaptureTarget> = None;

        card(
            ui,
            &pal,
            icons::KEYBOARD,
            t.hotkeys,
            |_| {},
            |ui| {
                let hk = &mut self.cfg.hotkeys;
                row(ui, &pal, t.hk_toggle, None, |ui| {
                    let capturing = capture == Some(CaptureTarget::Toggle);
                    if widgets::key_button(ui, &pal, &hk.toggle.label(), capturing, t.press_key, 170.0).clicked() {
                        begin = Some(CaptureTarget::Toggle);
                    }
                });
                let hint = (hk.mode == HotkeyMode::Hold).then_some(t.hk_hold_hint);
                row(ui, &pal, t.hk_mode, hint, |ui| {
                    segmented(
                        ui,
                        &pal,
                        &mut hk.mode,
                        &[(HotkeyMode::Toggle, t.hk_mode_toggle), (HotkeyMode::Hold, t.hk_mode_hold)],
                        false,
                    );
                });
                row(ui, &pal, t.hk_add_point, None, |ui| {
                    let capturing = capture == Some(CaptureTarget::AddPoint);
                    if widgets::key_button(ui, &pal, &hk.add_point.label(), capturing, t.press_key, 170.0).clicked() {
                        begin = Some(CaptureTarget::AddPoint);
                    }
                });
                widgets::hint(ui, &pal, t.hotkeys_hint);
            },
        );

        card(
            ui,
            &pal,
            icons::SHIELD_CHECK,
            t.safety,
            |_| {},
            |ui| {
                row(ui, &pal, t.start_delay, Some(t.start_delay_hint), |ui| {
                    num(ui, &mut self.cfg.start_delay_s, 0..=60, 84.0, &format!(" {}", t.unit_s));
                });
                row(ui, &pal, t.failsafe, Some(t.failsafe_hint), |ui| {
                    widgets::switch(ui, &pal, &mut self.cfg.failsafe);
                });
                widgets::icon_text(ui, icons::INFO, pal.text_dim, t.self_click_note, pal.text_dim, 12.5);
            },
        );

        card(
            ui,
            &pal,
            icons::PALETTE,
            t.interface,
            |_| {},
            |ui| {
                row(ui, &pal, t.language, None, |ui| {
                    segmented(ui, &pal, &mut self.cfg.language, &[(Lang::Ru, "Русский"), (Lang::En, "English")], false);
                });
                row(ui, &pal, t.theme, None, |ui| {
                    let dark = format!("{}  {}", icons::MOON, t.theme_dark);
                    let light = format!("{}  {}", icons::SUN, t.theme_light);
                    segmented(
                        ui,
                        &pal,
                        &mut self.cfg.theme,
                        &[(ThemeChoice::Dark, dark.as_str()), (ThemeChoice::Light, light.as_str())],
                        false,
                    );
                });
                row(ui, &pal, t.always_on_top, None, |ui| {
                    widgets::switch(ui, &pal, &mut self.cfg.always_on_top);
                });
            },
        );

        let total = self.controller.stats.total();
        let mut reset = false;
        card(
            ui,
            &pal,
            icons::CHART_LINE,
            t.statistics,
            |_| {},
            |ui| {
                row(ui, &pal, t.total_clicks, None, |ui| {
                    if widgets::button(ui, &pal, Some(icons::ARROW_COUNTER_CLOCKWISE), t.reset, ButtonKind::Secondary)
                        .clicked()
                    {
                        reset = true;
                    }
                    ui.label(RichText::new(group_digits(total)).font(super::theme::semibold(16.0)));
                });
            },
        );
        if reset {
            self.controller.stats.set_total(0);
            self.cfg.total_clicks = 0;
        }

        let path = self.config_path().display().to_string();
        let location = format!("{}: {path}", t.config_at);
        widgets::icon_text(ui, icons::FOLDER_SIMPLE, pal.text_faint, &location, pal.text_faint, 11.5);
        ui.label(RichText::new(concat!("clkr v", env!("CARGO_PKG_VERSION"))).size(11.5).color(pal.text_faint));

        if let Some(target) = begin {
            self.begin_capture(target);
        }
    }
}
