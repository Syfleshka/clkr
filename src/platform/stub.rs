//! Inert implementation for non-Windows systems: lets the UI run for
//! development and previews, but never touches real input.

use std::sync::Mutex;

use super::{MouseButton, PickRequest, PickResult, ScreenRect};

static CURSOR: Mutex<(i32, i32)> = Mutex::new((960, 540));

pub fn window_position() -> Option<(i32, i32)> {
    None
}

pub fn restore_window_position(_pos: (i32, i32)) {}

pub fn is_own_window_at(_x: i32, _y: i32) -> bool {
    false
}

pub fn own_window_focused() -> bool {
    false
}

pub fn cursor_pos() -> (i32, i32) {
    *CURSOR.lock().unwrap()
}

pub fn set_cursor_pos(x: i32, y: i32) {
    *CURSOR.lock().unwrap() = (x, y);
}

pub fn virtual_screen() -> ScreenRect {
    ScreenRect { x: 0, y: 0, w: 1920, h: 1080 }
}

pub fn monitors() -> Vec<ScreenRect> {
    vec![virtual_screen()]
}

pub fn mouse_button(_button: MouseButton, _down: bool, at: Option<(i32, i32)>) {
    if let Some((x, y)) = at {
        set_cursor_pos(x, y);
    }
}

pub fn mouse_move(x: i32, y: i32) {
    set_cursor_pos(x, y);
}

pub fn key_event(_vk: u16, _down: bool) {}

pub fn is_key_down(_vk: u16) -> bool {
    false
}

pub fn os_key_name(_vk: u16) -> Option<String> {
    None
}

pub struct HighResTimer(());

impl HighResTimer {
    pub fn new() -> Self {
        Self(())
    }
}

pub fn prefers_russian() -> bool {
    std::env::var("LANG").is_ok_and(|l| l.starts_with("ru"))
}

pub fn system_fonts() -> Option<super::SystemFonts> {
    let pairs = [
        ("/usr/share/fonts/opentype/inter/Inter-Regular.otf", "/usr/share/fonts/opentype/inter/Inter-SemiBold.otf"),
        ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
    ];
    pairs.iter().find_map(|(r, b)| {
        Some(super::SystemFonts {
            regular: std::fs::read(r).ok()?,
            semibold: std::fs::read(b).ok()?,
            fallbacks: Vec::new(),
        })
    })
}

pub fn pick_on_screen(req: PickRequest, done: impl FnOnce(PickResult) + Send + 'static) {
    // No overlay here: return something plausible so the flow can be exercised.
    let result = match req.kind {
        super::PickKind::Point => PickResult::Point(1280, 720),
        super::PickKind::Area => PickResult::Area(ScreenRect { x: 600, y: 300, w: 480, h: 320 }),
        super::PickKind::Points => PickResult::Points(vec![(400, 300), (900, 520), (1400, 360)]),
    };
    std::thread::spawn(move || done(result));
}
