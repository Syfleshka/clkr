//! The click engine: a dedicated thread that performs clicks with precise
//! timing, plus a thread-safe [`Controller`] used by the UI and the hotkeys.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::platform::{self, MouseButton};
use crate::settings::{ActionKind, ClickKind, HotkeyConfig, LimitMode, LocationMode, Problem, SeqOrder, Settings};

/// Pause between the two clicks of a double click.
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(30);
/// How far the user may move the mouse before the failsafe stops clicking.
const FAILSAFE_DISTANCE: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Idle,
    /// Start requested, engine hasn't picked it up yet.
    Starting,
    Countdown,
    Running,
}

impl RunState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Starting,
            2 => Self::Countdown,
            3 => Self::Running,
            _ => Self::Idle,
        }
    }

    pub fn is_active(self) -> bool {
        self != Self::Idle
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    User,
    LimitReached,
    MouseMoved,
}

/// Messages from background threads to the UI.
#[derive(Clone, Debug)]
pub enum AppEvent {
    Stopped(StopReason),
    StartRefused(Problem),
    /// The "add point" hotkey was pressed with the cursor at this position.
    AddPoint(i32, i32),
    /// A key was captured for the given target (`None` = cancelled).
    Captured(crate::hotkeys::CaptureTarget, Option<crate::keys::KeyCombo>),
    Picked(platform::PickResult),
}

enum Cmd {
    Start(Box<Settings>),
    Update(Box<Settings>),
    Stop,
}

#[derive(Default)]
struct Times {
    started: Option<Instant>,
    ended: Option<Instant>,
    countdown_end: Option<Instant>,
}

/// Live statistics, written by the engine and read by the UI.
#[derive(Default)]
pub struct Stats {
    state: AtomicU8,
    clicks: AtomicU64,
    total: AtomicU64,
    cycles: AtomicU64,
    seq_index: AtomicUsize,
    /// Clicks are skipped because the target is clkr's own window.
    blocked: AtomicBool,
    times: Mutex<Times>,
}

impl Stats {
    pub fn state(&self) -> RunState {
        RunState::from_u8(self.state.load(Ordering::Relaxed))
    }

    fn set_state(&self, s: RunState) {
        self.state.store(s as u8, Ordering::Relaxed);
    }

    /// Clicks in the current (or last) run.
    pub fn clicks(&self) -> u64 {
        self.clicks.load(Ordering::Relaxed)
    }

    pub fn cycles(&self) -> u64 {
        self.cycles.load(Ordering::Relaxed)
    }

    /// Clicks over the lifetime of the app.
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn set_total(&self, v: u64) {
        self.total.store(v, Ordering::Relaxed);
    }

    pub fn seq_index(&self) -> usize {
        self.seq_index.load(Ordering::Relaxed)
    }

    pub fn blocked(&self) -> bool {
        self.blocked.load(Ordering::Relaxed)
    }

    /// Duration of the current run, or of the last one when idle.
    pub fn elapsed(&self) -> Duration {
        let t = self.times.lock().unwrap();
        match (t.started, t.ended) {
            (Some(s), Some(e)) => e.saturating_duration_since(s),
            (Some(s), None) => s.elapsed(),
            _ => Duration::ZERO,
        }
    }

    pub fn countdown_left(&self) -> Option<Duration> {
        let t = self.times.lock().unwrap();
        t.countdown_end.map(|end| end.saturating_duration_since(Instant::now()))
    }
}

struct Live {
    settings: Settings,
    hotkeys: HotkeyConfig,
}

/// App-wide options the engine reads at run time.
#[derive(Default)]
struct Opts {
    failsafe: AtomicBool,
    start_delay_s: AtomicU32,
}

/// Thread-safe handle used by the UI and the hotkey thread.
pub struct Controller {
    tx: Sender<Cmd>,
    events: Sender<AppEvent>,
    ctx: egui::Context,
    pub stats: Arc<Stats>,
    opts: Arc<Opts>,
    live: Mutex<Live>,
}

impl Controller {
    pub fn new(ctx: egui::Context, events: Sender<AppEvent>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let stats = Arc::new(Stats::default());
        let opts = Arc::new(Opts::default());
        let engine = Engine { rx, stats: stats.clone(), opts: opts.clone(), events: events.clone(), ctx: ctx.clone() };
        std::thread::Builder::new()
            .name("clkr-engine".into())
            .spawn(move || engine.run())
            .expect("spawn engine thread");
        Arc::new(Self {
            tx,
            events,
            ctx,
            stats,
            opts,
            live: Mutex::new(Live { settings: Settings::default(), hotkeys: HotkeyConfig::default() }),
        })
    }

    /// Push the latest settings from the UI. Applied live if a run is active.
    pub fn update_live(&self, settings: &Settings, hotkeys: &HotkeyConfig, start_delay_s: u32, failsafe: bool) {
        self.opts.failsafe.store(failsafe, Ordering::Relaxed);
        self.opts.start_delay_s.store(start_delay_s, Ordering::Relaxed);
        let mut live = self.live.lock().unwrap();
        let changed = live.settings != *settings;
        live.settings = settings.clone();
        live.hotkeys = *hotkeys;
        if changed && self.state().is_active() {
            let _ = self.tx.send(Cmd::Update(Box::new(settings.clone())));
        }
    }

    pub fn state(&self) -> RunState {
        self.stats.state()
    }

    pub fn start(&self) {
        if self.state().is_active() {
            return;
        }
        let live = self.live.lock().unwrap();
        if let Some(problem) = live.settings.problem(&live.hotkeys) {
            let _ = self.events.send(AppEvent::StartRefused(problem));
            self.ctx.request_repaint();
            return;
        }
        self.stats.set_state(RunState::Starting);
        let _ = self.tx.send(Cmd::Start(Box::new(live.settings.clone())));
        self.ctx.request_repaint();
    }

    pub fn stop(&self) {
        if self.state().is_active() {
            let _ = self.tx.send(Cmd::Stop);
        }
    }

    pub fn toggle(&self) {
        if self.state().is_active() { self.stop() } else { self.start() }
    }
}

/// Small xorshift PRNG; good enough for jitter and random points.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9, |d| d.as_nanos() as u64);
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `0.0..1.0`.
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
}

struct Engine {
    rx: Receiver<Cmd>,
    stats: Arc<Stats>,
    opts: Arc<Opts>,
    events: Sender<AppEvent>,
    ctx: egui::Context,
}

enum Wait {
    Elapsed,
    Stop,
    Quit,
}

impl Engine {
    fn run(mut self) {
        while let Ok(cmd) = self.rx.recv() {
            if let Cmd::Start(settings) = cmd {
                let reason = self.run_job(*settings);
                {
                    let mut t = self.stats.times.lock().unwrap();
                    t.countdown_end = None;
                    if t.started.is_some() {
                        t.ended = Some(Instant::now());
                    }
                }
                self.stats.blocked.store(false, Ordering::Relaxed);
                self.stats.set_state(RunState::Idle);
                match reason {
                    Some(StopReason::User) => {}
                    Some(reason) => {
                        let _ = self.events.send(AppEvent::Stopped(reason));
                    }
                    None => return, // UI is gone
                }
                self.ctx.request_repaint();
            }
        }
    }

    /// Waits until `deadline` while staying responsive to commands.
    fn wait_until(&self, deadline: Instant, settings: &mut Settings) -> Wait {
        loop {
            let now = Instant::now();
            if now >= deadline {
                // Still honour a command that is already waiting (e.g. a quick stop).
                return match self.rx.try_recv() {
                    Ok(Cmd::Stop) => Wait::Stop,
                    Ok(Cmd::Update(s)) => {
                        *settings = *s;
                        Wait::Elapsed
                    }
                    Err(TryRecvError::Disconnected) => Wait::Quit,
                    Ok(Cmd::Start(_)) | Err(TryRecvError::Empty) => Wait::Elapsed,
                };
            }
            let left = deadline - now;
            // Sleep for the bulk of the wait, spin for the last ~2 ms for accuracy.
            let cmd = if left > Duration::from_millis(2) {
                match self.rx.recv_timeout(left - Duration::from_micros(1500)) {
                    Ok(cmd) => Some(cmd),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return Wait::Quit,
                }
            } else {
                std::thread::yield_now();
                self.rx.try_recv().ok()
            };
            match cmd {
                Some(Cmd::Stop) => return Wait::Stop,
                Some(Cmd::Update(s)) => *settings = *s,
                Some(Cmd::Start(_)) | None => {}
            }
        }
    }

    fn run_job(&mut self, mut s: Settings) -> Option<StopReason> {
        let stats = self.stats.clone();
        stats.clicks.store(0, Ordering::Relaxed);
        stats.cycles.store(0, Ordering::Relaxed);
        stats.seq_index.store(0, Ordering::Relaxed);
        *stats.times.lock().unwrap() = Times::default();

        let delay = self.opts.start_delay_s.load(Ordering::Relaxed);
        if delay > 0 {
            let end = Instant::now() + Duration::from_secs(delay as u64);
            stats.times.lock().unwrap().countdown_end = Some(end);
            stats.set_state(RunState::Countdown);
            self.ctx.request_repaint();
            match self.wait_until(end, &mut s) {
                Wait::Elapsed => {}
                Wait::Stop => return Some(StopReason::User),
                Wait::Quit => return None,
            }
            stats.times.lock().unwrap().countdown_end = None;
        }

        let _timer = platform::HighResTimer::new();
        let started = Instant::now();
        stats.times.lock().unwrap().started = Some(started);
        stats.set_state(RunState::Running);
        self.ctx.request_repaint();

        let mut rng = Rng::new();
        let mut seq_i = 0usize;
        // Where we last put the cursor (for the failsafe).
        let mut expected: Option<(i32, i32)> = None;
        let mut next = Instant::now();
        let mut last_click = next;

        loop {
            let end = (s.limit == LimitMode::Time).then(|| started + Duration::from_secs(s.limit_secs.max(1)));
            let deadline = end.map_or(next, |e| e.min(next));
            let before = s.interval_ms();
            match self.wait_until(deadline, &mut s) {
                Wait::Elapsed => {}
                Wait::Stop => return Some(StopReason::User),
                Wait::Quit => return None,
            }
            if s.interval_ms() != before {
                // Rate changed live: re-plan from the last click.
                next = last_click + Duration::from_secs_f64(s.interval_ms() / 1000.0);
                if Instant::now() < next {
                    continue;
                }
            }
            if end.is_some_and(|e| Instant::now() >= e) {
                return Some(StopReason::LimitReached);
            }

            if self.opts.failsafe.load(Ordering::Relaxed)
                && let Some(p) = expected
            {
                let c = platform::cursor_pos();
                if (c.0 - p.0).abs() > FAILSAFE_DISTANCE || (c.1 - p.1).abs() > FAILSAFE_DISTANCE {
                    return Some(StopReason::MouseMoved);
                }
            }

            if s.sequence.is_empty() {
                seq_i = 0;
            } else {
                seq_i %= s.sequence.len();
            }
            let (clicked, pause_ms) = perform(&s, seq_i, &mut rng, &mut expected);
            stats.blocked.store(!clicked, Ordering::Relaxed);
            last_click = Instant::now();

            if clicked {
                stats.clicks.fetch_add(1, Ordering::Relaxed);
                stats.total.fetch_add(1, Ordering::Relaxed);
                if s.action == ActionKind::Mouse && s.location == LocationMode::Sequence && !s.sequence.is_empty() {
                    seq_i = match s.seq_order {
                        SeqOrder::InOrder => {
                            let n = (seq_i + 1) % s.sequence.len();
                            if n == 0 {
                                stats.cycles.fetch_add(1, Ordering::Relaxed);
                            }
                            n
                        }
                        SeqOrder::Random => rng.below(s.sequence.len()),
                    };
                    stats.seq_index.store(seq_i, Ordering::Relaxed);
                }
            }

            if s.limit == LimitMode::Count {
                let done = if s.counts_cycles() { stats.cycles() } else { stats.clicks() };
                if done >= s.limit_count.max(1) {
                    return Some(StopReason::LimitReached);
                }
            }

            // Schedule the next click from the previous deadline so timing doesn't drift.
            let mut ms = s.interval_ms() + pause_ms as f64;
            if s.jitter > 0.0 {
                ms *= 1.0 + (rng.unit() * 2.0 - 1.0) * s.jitter as f64;
            }
            let step = Duration::from_secs_f64(ms.max(0.5) / 1000.0);
            next += step;
            let now = Instant::now();
            if next < now {
                // We fell behind (slow target app, long hold): don't burst to catch up.
                next = now;
            }
        }
    }
}

fn sleep_precise(d: Duration) {
    if d.is_zero() {
        return;
    }
    let end = Instant::now() + d;
    if d > Duration::from_millis(2) {
        std::thread::sleep(d - Duration::from_micros(1500));
    }
    while Instant::now() < end {
        std::thread::yield_now();
    }
}

fn click(button: MouseButton, kind: ClickKind, hold: Duration, at: Option<(i32, i32)>) {
    let times = if kind == ClickKind::Double { 2 } else { 1 };
    for i in 0..times {
        if i > 0 {
            sleep_precise(DOUBLE_CLICK_GAP);
        }
        platform::mouse_button(button, true, at);
        sleep_precise(hold);
        platform::mouse_button(button, false, at);
    }
}

/// Does one action. Returns whether anything was clicked (clicks aimed at clkr
/// itself are skipped) and the extra pause requested by a route point.
fn perform(s: &Settings, seq_i: usize, rng: &mut Rng, expected: &mut Option<(i32, i32)>) -> (bool, u32) {
    let hold = Duration::from_millis(s.hold_ms as u64);
    if s.action == ActionKind::Keyboard {
        if platform::own_window_focused() {
            return (false, 0);
        }
        let k = s.key;
        let mods: Vec<u16> =
            [(k.ctrl, crate::keys::VK_CONTROL), (k.shift, crate::keys::VK_SHIFT), (k.alt, crate::keys::VK_MENU)]
                .into_iter()
                .filter_map(|(on, vk)| on.then_some(vk))
                .collect();
        for &m in &mods {
            platform::key_event(m, true);
        }
        platform::key_event(k.vk, true);
        sleep_precise(hold);
        platform::key_event(k.vk, false);
        for &m in mods.iter().rev() {
            platform::key_event(m, false);
        }
        return (true, 0);
    }

    let (target, button, kind, pause) = match s.location {
        LocationMode::Cursor => (None, s.button, s.click, 0),
        LocationMode::Point => (Some(s.point), s.button, s.click, 0),
        LocationMode::Area => {
            let a = s.area;
            let x = a.x + (rng.unit() * a.w.max(1) as f64) as i32;
            let y = a.y + (rng.unit() * a.h.max(1) as f64) as i32;
            (Some((x, y)), s.button, s.click, 0)
        }
        LocationMode::Sequence => match s.sequence.get(seq_i) {
            Some(p) => (Some((p.x, p.y)), p.button, p.click, p.pause_ms),
            None => return (false, 0),
        },
    };

    let (x, y) = target.unwrap_or_else(platform::cursor_pos);
    if platform::is_own_window_at(x, y) {
        return (false, pause);
    }
    let restore = (s.return_cursor && target.is_some()).then(platform::cursor_pos);
    if let Some((tx, ty)) = target {
        platform::set_cursor_pos(tx, ty);
    }
    click(button, kind, hold, target);
    match restore {
        Some((rx, ry)) => {
            platform::mouse_move(rx, ry);
            *expected = None;
        }
        None => *expected = target,
    }
    (true, pause)
}
