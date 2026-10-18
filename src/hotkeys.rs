//! Global hotkeys and key capture.
//!
//! A background thread polls the key state every few milliseconds. Polling
//! (instead of `RegisterHotKey`) also allows mouse side buttons as hotkeys,
//! "hold to click" mode, and works while the window is minimised or hidden.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::engine::{AppEvent, Controller};
use crate::keys::{self, KeyCombo};
use crate::platform;
use crate::settings::{HotkeyConfig, HotkeyMode};

const POLL: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureTarget {
    Toggle,
    AddPoint,
    ActionKey,
}

#[derive(Default)]
struct State {
    cfg: HotkeyConfig,
    capture: Option<CaptureTarget>,
}

pub struct Hotkeys {
    state: Mutex<State>,
}

impl Hotkeys {
    pub fn spawn(controller: Arc<Controller>, events: Sender<AppEvent>, ctx: egui::Context) -> Arc<Self> {
        let me = Arc::new(Self { state: Mutex::new(State::default()) });
        let shared = me.clone();
        std::thread::Builder::new()
            .name("clkr-hotkeys".into())
            .spawn(move || poll_loop(shared, controller, events, ctx))
            .expect("spawn hotkey thread");
        me
    }

    pub fn set_config(&self, cfg: HotkeyConfig) {
        self.state.lock().unwrap().cfg = cfg;
    }

    /// Wait for the next key press and report it as [`AppEvent::Captured`].
    pub fn begin_capture(&self, target: CaptureTarget) {
        self.state.lock().unwrap().capture = Some(target);
    }

    pub fn cancel_capture(&self) {
        self.state.lock().unwrap().capture = None;
    }
}

fn modifiers() -> (bool, bool, bool) {
    (
        platform::is_key_down(keys::VK_CONTROL),
        platform::is_key_down(keys::VK_SHIFT),
        platform::is_key_down(keys::VK_MENU),
    )
}

fn combo_down(c: &KeyCombo) -> bool {
    c.is_set() && platform::is_key_down(c.vk) && modifiers() == (c.ctrl, c.shift, c.alt)
}

fn capturable(vk: u16, target: CaptureTarget) -> bool {
    // Left/right mouse buttons are never allowed: the clicker itself sends them.
    if vk <= 0x03 || keys::is_modifier(vk) || vk == keys::VK_ESCAPE {
        return false;
    }
    let mouse = matches!(vk, keys::VK_MBUTTON | keys::VK_XBUTTON1 | keys::VK_XBUTTON2);
    !(mouse && target == CaptureTarget::ActionKey)
}

fn poll_loop(shared: Arc<Hotkeys>, controller: Arc<Controller>, events: Sender<AppEvent>, ctx: egui::Context) {
    let mut prev_toggle = false;
    let mut prev_add = false;
    let mut prev_keys = [false; 256];
    let mut capturing = false;

    loop {
        std::thread::sleep(POLL);
        let (cfg, capture) = {
            let s = shared.state.lock().unwrap();
            (s.cfg, s.capture)
        };

        if let Some(target) = capture {
            let mut found: Option<Option<KeyCombo>> = None;
            for vk in 1u16..=254 {
                let down = platform::is_key_down(vk);
                let was = std::mem::replace(&mut prev_keys[vk as usize], down);
                // The first scan only records what is already held down.
                if !capturing || !down || was || found.is_some() {
                    continue;
                }
                if vk == keys::VK_ESCAPE {
                    found = Some(None);
                } else if capturable(vk, target) {
                    let (ctrl, shift, alt) = modifiers();
                    found = Some(Some(KeyCombo { vk, ctrl, shift, alt }));
                }
            }
            capturing = true;
            if let Some(result) = found {
                shared.state.lock().unwrap().capture = None;
                capturing = false;
                let _ = events.send(AppEvent::Captured(target, result));
                ctx.request_repaint();
            }
            // Don't fire hotkeys while the user is assigning one.
            prev_toggle = true;
            prev_add = true;
            continue;
        }
        capturing = false;

        let toggle = combo_down(&cfg.toggle);
        match cfg.mode {
            HotkeyMode::Toggle if toggle && !prev_toggle => controller.toggle(),
            HotkeyMode::Hold if toggle && !prev_toggle => controller.start(),
            HotkeyMode::Hold if !toggle && prev_toggle => controller.stop(),
            _ => {}
        }
        prev_toggle = toggle;

        let add = combo_down(&cfg.add_point);
        if add && !prev_add {
            let (x, y) = platform::cursor_pos();
            let _ = events.send(AppEvent::AddPoint(x, y));
            ctx.request_repaint();
        }
        prev_add = add;
    }
}
