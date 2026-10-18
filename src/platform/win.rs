//! Windows implementation of the OS layer (Win32 via `windows-sys`).

use std::cell::Cell;
use std::mem::size_of;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
use windows_sys::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyNameTextW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, MapVirtualKeyW, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CallWindowProcW, GA_ROOT, GWLP_WNDPROC, GetAncestor, GetCursorPos, GetForegroundWindow,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, IsIconic, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SetCursorPos, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOWPOS, WM_DPICHANGED,
    WM_WINDOWPOSCHANGING, WNDPROC, WindowFromPoint,
};

use super::{MouseButton, PickRequest, PickResult, ScreenRect};
use crate::keys;

/// Marks input events injected by clkr.
const CLKR_TAG: usize = 0x434C_4B52;

static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
/// winit's window procedure, wrapped by [`dpi_fix_proc`].
static WINIT_WNDPROC: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    /// Window rectangle suggested by Windows while a `WM_DPICHANGED` is being handled.
    static DPI_RECT: Cell<Option<RECT>> = const { Cell::new(None) };
}

pub(super) fn main_hwnd() -> HWND {
    MAIN_HWND.load(Ordering::Relaxed) as HWND
}

/// Remember the main window, give it Windows 11 rounded corners and fix how it
/// reacts to moving between monitors with different scaling.
///
/// Must be called on the thread that owns the window.
pub fn init_main_window(hwnd: isize) {
    MAIN_HWND.store(hwnd, Ordering::Relaxed);
    let pref = DWMWCP_ROUND;
    unsafe {
        DwmSetWindowAttribute(
            hwnd as HWND,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &pref as *const i32 as *const _,
            size_of::<i32>() as u32,
        );
        // Store the original procedure before swapping it in, so it is never called unset.
        WINIT_WNDPROC.store(GetWindowLongPtrW(hwnd as HWND, GWLP_WNDPROC), Ordering::Relaxed);
        let proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = dpi_fix_proc;
        SetWindowLongPtrW(hwnd as HWND, GWLP_WNDPROC, proc as usize as isize);
    }
}

/// Wraps winit's window procedure to handle DPI changes the way Windows expects.
///
/// On `WM_DPICHANGED` winit positions the borderless window itself (and counts
/// a border it doesn't have) instead of using the rectangle Windows suggests.
/// That rectangle can reach back onto the previous monitor, which immediately
/// triggers the opposite DPI change; while dragging between monitors with
/// different scaling the window then flickers and drifts in size. Here winit
/// still handles the message (so egui rescales), but the move it makes is
/// replaced by the suggested rectangle, which always lies on the new monitor.
unsafe extern "system" fn dpi_fix_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let winit: WNDPROC = std::mem::transmute::<isize, WNDPROC>(WINIT_WNDPROC.load(Ordering::Relaxed));
        match msg {
            WM_DPICHANGED => {
                DPI_RECT.set(Some(*(lparam as *const RECT)));
                let result = CallWindowProcW(winit, hwnd, msg, wparam, lparam);
                DPI_RECT.set(None);
                result
            }
            WM_WINDOWPOSCHANGING => {
                if let Some(r) = DPI_RECT.get() {
                    let pos = &mut *(lparam as *mut WINDOWPOS);
                    pos.x = r.left;
                    pos.y = r.top;
                    pos.cx = r.right - r.left;
                    pos.cy = r.bottom - r.top;
                    pos.flags &= !(SWP_NOMOVE | SWP_NOSIZE);
                }
                CallWindowProcW(winit, hwnd, msg, wparam, lparam)
            }
            _ => CallWindowProcW(winit, hwnd, msg, wparam, lparam),
        }
    }
}

/// Outer position of the main window in physical pixels.
pub fn window_position() -> Option<(i32, i32)> {
    let hwnd = main_hwnd();
    // A minimised window reports a bogus position far off screen.
    if hwnd.is_null() || unsafe { IsIconic(hwnd) } != 0 {
        return None;
    }
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    (unsafe { GetWindowRect(hwnd, &mut r) } != 0).then_some((r.left, r.top))
}

/// Move the main window, but only if the spot is still on a connected monitor.
pub fn restore_window_position(pos: (i32, i32)) {
    let hwnd = main_hwnd();
    let probe = (pos.0 + 60, pos.1 + 16);
    if hwnd.is_null() || !monitors().iter().any(|m| m.contains(probe)) {
        return;
    }
    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), pos.0, pos.1, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn hide_main_window() {
    let hwnd = main_hwnd();
    if !hwnd.is_null() {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

pub fn show_main_window() {
    let hwnd = main_hwnd();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
    }
}

/// Is the pixel covered by one of clkr's own windows?
pub fn is_own_window_at(x: i32, y: i32) -> bool {
    let main = main_hwnd();
    if main.is_null() {
        return false;
    }
    unsafe {
        let hit = WindowFromPoint(POINT { x, y });
        !hit.is_null() && GetAncestor(hit, GA_ROOT) == main
    }
}

pub fn own_window_focused() -> bool {
    let main = main_hwnd();
    !main.is_null() && unsafe { GetForegroundWindow() } == main
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

pub fn set_cursor_pos(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y) };
}

pub fn virtual_screen() -> ScreenRect {
    unsafe {
        ScreenRect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        }
    }
}

pub fn monitors() -> Vec<ScreenRect> {
    unsafe extern "system" fn collect(_: HMONITOR, _: HDC, rect: *mut RECT, data: LPARAM) -> i32 {
        unsafe {
            let list = &mut *(data as *mut Vec<ScreenRect>);
            let r = &*rect;
            list.push(ScreenRect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top });
        }
        1
    }
    let mut list: Vec<ScreenRect> = Vec::new();
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect), &mut list as *mut _ as LPARAM);
    }
    if list.is_empty() {
        list.push(virtual_screen());
    }
    list
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
    }
}

/// Map a physical pixel to the 0..=65535 range used by absolute mouse input.
fn normalize(x: i32, y: i32) -> (i32, i32) {
    let vs = virtual_screen();
    let map = |v: i32, origin: i32, size: i32| -> i32 {
        let size = size as i64;
        ((((v - origin) as i64) * 65536 + size - 1) / size).clamp(0, 65535) as i32
    };
    (map(x, vs.x, vs.w), map(y, vs.y, vs.h))
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS, at: Option<(i32, i32)>) -> INPUT {
    let mut mi = MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: CLKR_TAG };
    if let Some((x, y)) = at {
        let (nx, ny) = normalize(x, y);
        mi.dx = nx;
        mi.dy = ny;
        mi.dwFlags |= MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
    }
    INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi } }
}

/// Press or release a mouse button. With `at`, the event carries its own
/// absolute position so it can't race with later cursor moves.
pub fn mouse_button(button: MouseButton, down: bool, at: Option<(i32, i32)>) {
    let flags = match (button, down) {
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
    };
    send(&[mouse_input(flags, at)]);
}

/// Move the cursor through the input stream (keeps ordering with clicks).
pub fn mouse_move(x: i32, y: i32) {
    send(&[mouse_input(0, Some((x, y)))]);
}

pub fn key_event(vk: u16, down: bool) {
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
    // Scan codes work in games that read raw/DirectInput keyboard state.
    let mut flags = if scan != 0 { KEYEVENTF_SCANCODE } else { 0 };
    if keys::is_extended(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    let ki =
        KEYBDINPUT { wVk: if scan != 0 { 0 } else { vk }, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: CLKR_TAG };
    send(&[INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki } }]);
}

pub fn is_key_down(vk: u16) -> bool {
    (unsafe { GetAsyncKeyState(vk as i32) } as u16) & 0x8000 != 0
}

/// Localised key name from the keyboard layout, for keys without a fixed name.
pub fn os_key_name(vk: u16) -> Option<String> {
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) };
    if scan == 0 {
        return None;
    }
    let mut lparam = (scan << 16) as i32;
    if keys::is_extended(vk) {
        lparam |= 1 << 24;
    }
    let mut buf = [0u16; 64];
    let len = unsafe { GetKeyNameTextW(lparam, buf.as_mut_ptr(), buf.len() as i32) };
    (len > 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

/// Raises the system timer resolution to 1 ms while alive, so sleeps between
/// clicks are precise.
pub struct HighResTimer(());

impl HighResTimer {
    pub fn new() -> Self {
        unsafe { timeBeginPeriod(1) };
        Self(())
    }
}

impl Drop for HighResTimer {
    fn drop(&mut self) {
        unsafe { timeEndPeriod(1) };
    }
}

pub fn prefers_russian() -> bool {
    const LANG_RUSSIAN: u16 = 0x19;
    (unsafe { GetUserDefaultUILanguage() } & 0x3FF) == LANG_RUSSIAN
}

/// UI fonts from the Windows font folder: Segoe UI, with older fallbacks.
pub fn system_fonts() -> Option<super::SystemFonts> {
    let dir =
        std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let first = |names: &[&str]| names.iter().find_map(|n| std::fs::read(dir.join(n)).ok());
    Some(super::SystemFonts {
        regular: first(&["segoeui.ttf", "arial.ttf", "tahoma.ttf"])?,
        semibold: first(&["seguisb.ttf", "segoeuib.ttf", "arialbd.ttf", "tahomabd.ttf"])?,
        // Symbols and pictographs that may appear in profile names.
        fallbacks: ["seguisym.ttf"].iter().filter_map(|n| std::fs::read(dir.join(n)).ok()).collect(),
    })
}

/// Show the full-screen picker on a background thread; `done` gets the result.
pub fn pick_on_screen(req: PickRequest, done: impl FnOnce(PickResult) + Send + 'static) {
    std::thread::spawn(move || done(super::picker_win::run(req)));
}
