//! Colours, fonts and the egui style.

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle};

use crate::settings::ThemeChoice;

pub const SEMIBOLD: &str = "semibold";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// Window background.
    pub bg: Color32,
    /// Cards.
    pub surface: Color32,
    /// Inputs, segmented tracks.
    pub field: Color32,
    /// Hovered fields / selected segment thumb.
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub danger: Color32,
    pub danger2: Color32,
    pub warning: Color32,
}

impl Palette {
    pub fn get(theme: ThemeChoice) -> Self {
        match theme {
            ThemeChoice::Dark => Self::DARK,
            ThemeChoice::Light => Self::LIGHT,
        }
    }

    pub const DARK: Self = Self {
        dark: true,
        bg: Color32::from_rgb(13, 15, 21),
        surface: Color32::from_rgb(21, 24, 33),
        field: Color32::from_rgb(29, 33, 44),
        raised: Color32::from_rgb(44, 49, 64),
        border: Color32::from_rgb(37, 42, 56),
        text: Color32::from_rgb(234, 236, 243),
        text_dim: Color32::from_rgb(142, 149, 168),
        text_faint: Color32::from_rgb(92, 99, 118),
        accent: Color32::from_rgb(124, 108, 255),
        accent2: Color32::from_rgb(64, 140, 255),
        accent_soft: Color32::from_rgba_premultiplied(25, 22, 51, 51),
        on_accent: Color32::WHITE,
        success: Color32::from_rgb(52, 211, 153),
        danger: Color32::from_rgb(244, 91, 105),
        danger2: Color32::from_rgb(255, 138, 76),
        warning: Color32::from_rgb(245, 185, 74),
    };

    pub const LIGHT: Self = Self {
        dark: false,
        bg: Color32::from_rgb(242, 243, 248),
        surface: Color32::from_rgb(255, 255, 255),
        field: Color32::from_rgb(237, 239, 245),
        raised: Color32::from_rgb(255, 255, 255),
        border: Color32::from_rgb(225, 228, 238),
        text: Color32::from_rgb(22, 25, 35),
        text_dim: Color32::from_rgb(98, 105, 125),
        text_faint: Color32::from_rgb(160, 166, 184),
        accent: Color32::from_rgb(103, 84, 245),
        accent2: Color32::from_rgb(45, 120, 245),
        accent_soft: Color32::from_rgba_premultiplied(12, 10, 29, 30),
        on_accent: Color32::WHITE,
        success: Color32::from_rgb(16, 170, 115),
        danger: Color32::from_rgb(232, 64, 84),
        danger2: Color32::from_rgb(246, 120, 56),
        warning: Color32::from_rgb(214, 145, 20),
    };
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("icons".into(), Arc::new(FontData::from_static(include_bytes!("../../assets/phosphor-subset.ttf"))));

    // Icons come first: they only cover private-use codepoints, which some
    // text fonts also fill with their own symbols.
    let mut regular = vec!["icons".to_owned()];
    let mut semibold = vec!["icons".to_owned()];
    let mut mono = vec!["icons".to_owned()];
    // egui's bundled fonts (only compiled in on non-Windows builds).
    let bundled = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let bundled_mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let mut fallbacks = Vec::new();
    if let Some(sys) = crate::platform::system_fonts() {
        fonts.font_data.insert("ui".into(), Arc::new(FontData::from_owned(sys.regular)));
        fonts.font_data.insert("ui-semibold".into(), Arc::new(FontData::from_owned(sys.semibold)));
        regular.push("ui".to_owned());
        semibold.push("ui-semibold".to_owned());
        for (i, data) in sys.fallbacks.into_iter().enumerate() {
            let name = format!("fallback-{i}");
            fonts.font_data.insert(name.clone(), Arc::new(FontData::from_owned(data)));
            fallbacks.push(name);
        }
    }
    mono.extend(bundled_mono);
    mono.push("ui".to_owned());
    for family in [&mut regular, &mut semibold, &mut mono] {
        family.extend(bundled.iter().cloned());
        family.extend(fallbacks.iter().cloned());
        family.retain(|name| fonts.font_data.contains_key(name));
    }

    fonts.families.insert(FontFamily::Proportional, regular);
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    fonts.families.insert(FontFamily::Monospace, mono);
    ctx.set_fonts(fonts);
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn style(pal: &Palette) -> egui::Style {
    let mut style = egui::Style {
        text_styles: [
            (TextStyle::Small, regular(12.0)),
            (TextStyle::Body, regular(14.0)),
            (TextStyle::Button, regular(14.0)),
            (TextStyle::Heading, semibold(18.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into(),
        animation_time: 0.14,
        ..Default::default()
    };

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(12.0, 6.0);
    s.interact_size = egui::vec2(36.0, 30.0);
    s.slider_width = 160.0;
    s.slider_rail_height = 6.0;
    s.menu_margin = Margin::same(6);
    s.combo_height = 300.0;
    s.scroll.bar_width = 6.0;
    s.scroll.floating = true;

    let v = &mut style.visuals;
    *v = if pal.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.override_text_color = Some(pal.text);
    v.panel_fill = pal.bg;
    v.window_fill = pal.surface;
    v.window_stroke = Stroke::new(1.0, pal.border);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(12);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if pal.dark { 120 } else { 40 }),
    };
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(if pal.dark { 110 } else { 34 }),
    };
    v.extreme_bg_color = pal.field;
    v.text_edit_bg_color = Some(pal.field);
    v.faint_bg_color = pal.field;
    v.hyperlink_color = pal.accent;
    v.selection.bg_fill = pal.accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, pal.accent);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.striped = false;

    let r = CornerRadius::same(9);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = pal.surface;
    w.noninteractive.weak_bg_fill = pal.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, pal.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, pal.text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = pal.field;
    w.inactive.weak_bg_fill = pal.field;
    w.inactive.bg_stroke = Stroke::new(1.0, pal.border);
    w.inactive.fg_stroke = Stroke::new(1.0, pal.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = pal.raised;
    w.hovered.weak_bg_fill = pal.raised;
    w.hovered.bg_stroke = Stroke::new(1.0, pal.accent.gamma_multiply(0.6));
    w.hovered.fg_stroke = Stroke::new(1.5, pal.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = pal.raised;
    w.active.weak_bg_fill = pal.raised;
    w.active.bg_stroke = Stroke::new(1.0, pal.accent);
    w.active.fg_stroke = Stroke::new(1.5, pal.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open = w.active;
    style
}

/// Horizontal gradient texture used for the big start/stop button.
pub fn gradient(ctx: &egui::Context, name: &str, a: Color32, b: Color32) -> egui::TextureHandle {
    const W: usize = 128;
    let pixels: Vec<Color32> = (0..W).map(|i| lerp(a, b, i as f32 / (W - 1) as f32)).collect();
    let image = egui::ColorImage::new([W, 1], pixels);
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}

pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}
