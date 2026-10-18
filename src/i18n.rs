//! UI strings in English and Russian.
//!
//! Every string is a field of [`Tr`], so a missing translation is a compile error.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

pub fn tr(lang: Lang) -> &'static Tr {
    match lang {
        Lang::En => &EN,
        Lang::Ru => &RU,
    }
}

pub struct Tr {
    pub lang: Lang,

    // Title bar
    pub tip_language: &'static str,
    pub tip_theme_light: &'static str,
    pub tip_theme_dark: &'static str,
    pub tip_pin: &'static str,
    pub tip_unpin: &'static str,
    pub tip_compact: &'static str,
    pub tip_expand: &'static str,
    pub tip_minimize: &'static str,
    pub tip_close: &'static str,

    // Profiles
    pub profiles: &'static str,
    pub profile_new: &'static str,
    pub profile_duplicate: &'static str,
    pub profile_rename: &'static str,
    pub profile_delete: &'static str,
    pub profile_default_name: &'static str,
    pub profile_copy_suffix: &'static str,
    pub rename_title: &'static str,
    pub delete_title: &'static str,
    pub save: &'static str,
    pub cancel: &'static str,
    pub delete: &'static str,

    // Hero
    pub status_ready: &'static str,
    pub status_starting: &'static str,
    pub status_countdown: &'static str,
    pub status_running: &'static str,
    pub status_paused_mouse: &'static str,
    pub status_paused_key: &'static str,
    pub stat_time: &'static str,
    pub stat_cps: &'static str,
    pub start: &'static str,
    pub stop: &'static str,
    pub of: &'static str,

    // Problems
    pub problem_no_key: &'static str,
    pub problem_key_is_hotkey: &'static str,
    pub problem_no_points: &'static str,
    pub problem_empty_area: &'static str,

    // Tabs
    pub tab_click: &'static str,
    pub tab_location: &'static str,
    pub tab_settings: &'static str,

    // Speed
    pub speed: &'static str,
    pub rate_cps: &'static str,
    pub rate_interval: &'static str,
    pub unit_h: &'static str,
    pub unit_min: &'static str,
    pub unit_s: &'static str,
    pub unit_ms: &'static str,
    pub every: &'static str,
    pub jitter: &'static str,
    pub jitter_hint: &'static str,

    // Action
    pub action: &'static str,
    pub mouse: &'static str,
    pub keyboard: &'static str,
    pub button: &'static str,
    pub btn_left: &'static str,
    pub btn_middle: &'static str,
    pub btn_right: &'static str,
    pub click_type: &'static str,
    pub single: &'static str,
    pub double: &'static str,
    pub key: &'static str,
    pub key_hint: &'static str,
    pub press_key: &'static str,
    pub hold: &'static str,
    pub hold_hint: &'static str,

    // Limit
    pub stop_after: &'static str,
    pub never: &'static str,
    pub count: &'static str,
    pub time: &'static str,
    pub never_hint: &'static str,

    // Location
    pub loc_cursor: &'static str,
    pub loc_point: &'static str,
    pub loc_area: &'static str,
    pub loc_route: &'static str,
    pub loc_cursor_desc: &'static str,
    pub loc_point_desc: &'static str,
    pub loc_area_desc: &'static str,
    pub loc_route_desc: &'static str,
    pub loc_keyboard_note: &'static str,
    pub pick_on_screen: &'static str,
    pub select_on_screen: &'static str,
    pub width: &'static str,
    pub height: &'static str,
    pub add_on_screen: &'static str,
    pub add_at_cursor: &'static str,
    pub in_order: &'static str,
    pub random: &'static str,
    pub clear: &'static str,
    pub route_empty: &'static str,
    pub pause: &'static str,
    pub tip_up: &'static str,
    pub tip_down: &'static str,
    pub tip_remove: &'static str,
    pub return_cursor: &'static str,
    pub return_cursor_hint: &'static str,

    // Settings
    pub hotkeys: &'static str,
    pub hotkeys_hint: &'static str,
    pub hk_toggle: &'static str,
    pub hk_mode: &'static str,
    pub hk_mode_toggle: &'static str,
    pub hk_mode_hold: &'static str,
    pub hk_hold_hint: &'static str,
    pub hk_add_point: &'static str,
    pub safety: &'static str,
    pub start_delay: &'static str,
    pub start_delay_hint: &'static str,
    pub failsafe: &'static str,
    pub failsafe_hint: &'static str,
    pub self_click_note: &'static str,
    pub interface: &'static str,
    pub language: &'static str,
    pub theme: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub always_on_top: &'static str,
    pub statistics: &'static str,
    pub total_clicks: &'static str,
    pub reset: &'static str,
    pub config_at: &'static str,

    // Notices
    pub stopped_limit: &'static str,
    pub stopped_mouse: &'static str,
    pub point_added: &'static str,
    pub points_added: &'static str,

    // Picker overlay
    pub pick_hint_point: &'static str,
    pub pick_hint_area: &'static str,
    pub pick_hint_points: &'static str,
}

impl Tr {
    /// "click"/"clicks" with the right plural form.
    pub fn clicks_word(&self, n: u64) -> &'static str {
        match self.lang {
            Lang::En => {
                if n == 1 {
                    "click"
                } else {
                    "clicks"
                }
            }
            Lang::Ru => ru_plural(n, "клик", "клика", "кликов"),
        }
    }

    pub fn cycles_word(&self, n: u64) -> &'static str {
        match self.lang {
            Lang::En => {
                if n == 1 {
                    "cycle"
                } else {
                    "cycles"
                }
            }
            Lang::Ru => ru_plural(n, "цикл", "цикла", "циклов"),
        }
    }

    pub fn points_word(&self, n: u64) -> &'static str {
        match self.lang {
            Lang::En => {
                if n == 1 {
                    "point"
                } else {
                    "points"
                }
            }
            Lang::Ru => ru_plural(n, "точка", "точки", "точек"),
        }
    }
}

fn ru_plural(n: u64, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
    let (n10, n100) = (n % 10, n % 100);
    if n10 == 1 && n100 != 11 {
        one
    } else if (2..=4).contains(&n10) && !(12..=14).contains(&n100) {
        few
    } else {
        many
    }
}

pub static EN: Tr = Tr {
    lang: Lang::En,

    tip_language: "Language",
    tip_theme_light: "Light theme",
    tip_theme_dark: "Dark theme",
    tip_pin: "Keep on top",
    tip_unpin: "Don't keep on top",
    tip_compact: "Compact mode",
    tip_expand: "Full mode",
    tip_minimize: "Minimize",
    tip_close: "Close",

    profiles: "Profiles",
    profile_new: "New profile",
    profile_duplicate: "Duplicate",
    profile_rename: "Rename",
    profile_delete: "Delete",
    profile_default_name: "Profile",
    profile_copy_suffix: "copy",
    rename_title: "Rename profile",
    delete_title: "Delete this profile?",
    save: "Save",
    cancel: "Cancel",
    delete: "Delete",

    status_ready: "Ready",
    status_starting: "Starting…",
    status_countdown: "Starting in",
    status_running: "Clicking",
    status_paused_mouse: "Paused: cursor is over clkr",
    status_paused_key: "Paused: clkr window is active",
    stat_time: "time",
    stat_cps: "clicks/s",
    start: "Start",
    stop: "Stop",
    of: "of",

    problem_no_key: "Choose a key to press",
    problem_key_is_hotkey: "The action key is also a hotkey",
    problem_no_points: "Add at least one route point",
    problem_empty_area: "The area is empty: select it on screen",

    tab_click: "Click",
    tab_location: "Location",
    tab_settings: "Settings",

    speed: "Speed",
    rate_cps: "Clicks/s",
    rate_interval: "Interval",
    unit_h: "h",
    unit_min: "min",
    unit_s: "s",
    unit_ms: "ms",
    every: "every",
    jitter: "Randomize",
    jitter_hint: "Irregular, human-like timing",

    action: "Action",
    mouse: "Mouse",
    keyboard: "Keyboard",
    button: "Button",
    btn_left: "Left",
    btn_middle: "Middle",
    btn_right: "Right",
    click_type: "Click",
    single: "Single",
    double: "Double",
    key: "Key",
    key_hint: "Ctrl / Shift / Alt combos work too",
    press_key: "Press a key…",
    hold: "Hold time",
    hold_hint: "How long the button stays pressed",

    stop_after: "Stop after",
    never: "Never",
    count: "Count",
    time: "Time",
    never_hint: "Runs until you stop it with the button or hotkey",

    loc_cursor: "Cursor",
    loc_point: "Point",
    loc_area: "Area",
    loc_route: "Route",
    loc_cursor_desc: "Clicks wherever the cursor is. The mouse stays yours.",
    loc_point_desc: "Always clicks the same spot on the screen.",
    loc_area_desc: "Clicks a random spot inside a rectangle every time.",
    loc_route_desc: "Clicks points in turn; each has its own button and pause.",
    loc_keyboard_note: "The action is a key press, so the location isn't used.",
    pick_on_screen: "Pick on screen",
    select_on_screen: "Select on screen",
    width: "Width",
    height: "Height",
    add_on_screen: "Add on screen",
    add_at_cursor: "At cursor",
    in_order: "In order",
    random: "Random",
    clear: "Clear",
    route_empty: "No points yet. Add them on screen or press the hotkey with the cursor over the target.",
    pause: "pause",
    tip_up: "Move up",
    tip_down: "Move down",
    tip_remove: "Remove",
    return_cursor: "Return the cursor",
    return_cursor_hint: "Moves the cursor back after every click",

    hotkeys: "Hotkeys",
    hotkeys_hint: "Global: they work even when clkr is minimized.",
    hk_toggle: "Start / stop",
    hk_mode: "Mode",
    hk_mode_toggle: "Toggle",
    hk_mode_hold: "Hold",
    hk_hold_hint: "Clicks only while the key is held",
    hk_add_point: "Add route point",
    safety: "Start & safety",
    start_delay: "Start delay",
    start_delay_hint: "Time to move the cursor into place",
    failsafe: "Stop if the mouse moves",
    failsafe_hint: "In point, area and route modes",
    self_click_note: "clkr never clicks its own window: clicks pause while the cursor is over it.",
    interface: "Interface",
    language: "Language",
    theme: "Theme",
    theme_dark: "Dark",
    theme_light: "Light",
    always_on_top: "Always on top",
    statistics: "Statistics",
    total_clicks: "Total clicks",
    reset: "Reset",
    config_at: "Settings file",

    stopped_limit: "Done: the limit was reached",
    stopped_mouse: "Stopped: the mouse was moved",
    point_added: "Route point added",
    points_added: "Added to the route:",

    pick_hint_point: "Click to pick a point   ·   arrows fine-tune   ·   Esc cancels",
    pick_hint_area: "Drag to select an area   ·   Esc cancels",
    pick_hint_points: "Click points in order   ·   Backspace undoes   ·   Enter or right-click finishes",
};

pub static RU: Tr = Tr {
    lang: Lang::Ru,

    tip_language: "Язык",
    tip_theme_light: "Светлая тема",
    tip_theme_dark: "Тёмная тема",
    tip_pin: "Поверх всех окон",
    tip_unpin: "Не поверх окон",
    tip_compact: "Компактный режим",
    tip_expand: "Полный режим",
    tip_minimize: "Свернуть",
    tip_close: "Закрыть",

    profiles: "Профили",
    profile_new: "Новый профиль",
    profile_duplicate: "Дублировать",
    profile_rename: "Переименовать",
    profile_delete: "Удалить",
    profile_default_name: "Профиль",
    profile_copy_suffix: "копия",
    rename_title: "Переименовать профиль",
    delete_title: "Удалить этот профиль?",
    save: "Сохранить",
    cancel: "Отмена",
    delete: "Удалить",

    status_ready: "Готов",
    status_starting: "Запуск…",
    status_countdown: "Старт через",
    status_running: "Кликает",
    status_paused_mouse: "Пауза: курсор над окном clkr",
    status_paused_key: "Пауза: окно clkr активно",
    stat_time: "время",
    stat_cps: "кликов/с",
    start: "Старт",
    stop: "Стоп",
    of: "из",

    problem_no_key: "Выберите клавишу для нажатия",
    problem_key_is_hotkey: "Клавиша действия занята горячей клавишей",
    problem_no_points: "Добавьте хотя бы одну точку маршрута",
    problem_empty_area: "Область пустая: выделите её на экране",

    tab_click: "Клик",
    tab_location: "Где кликать",
    tab_settings: "Настройки",

    speed: "Скорость",
    rate_cps: "Кликов/с",
    rate_interval: "Интервал",
    unit_h: "ч",
    unit_min: "мин",
    unit_s: "с",
    unit_ms: "мс",
    every: "каждые",
    jitter: "Разброс",
    jitter_hint: "Неровный ритм, как у человека",

    action: "Действие",
    mouse: "Мышь",
    keyboard: "Клавиатура",
    button: "Кнопка",
    btn_left: "Левая",
    btn_middle: "Средняя",
    btn_right: "Правая",
    click_type: "Клик",
    single: "Одинарный",
    double: "Двойной",
    key: "Клавиша",
    key_hint: "Можно с Ctrl / Shift / Alt",
    press_key: "Нажмите клавишу…",
    hold: "Удержание",
    hold_hint: "Сколько держать кнопку нажатой",

    stop_after: "Остановить",
    never: "Никогда",
    count: "По счёту",
    time: "По времени",
    never_hint: "Работает, пока не остановите кнопкой или горячей клавишей",

    loc_cursor: "Курсор",
    loc_point: "Точка",
    loc_area: "Зона",
    loc_route: "Маршрут",
    loc_cursor_desc: "Кликает там, где сейчас курсор. Мышь остаётся вашей.",
    loc_point_desc: "Всегда кликает в одно и то же место экрана.",
    loc_area_desc: "Каждый раз кликает в случайную точку внутри прямоугольника.",
    loc_route_desc: "Кликает точки по очереди, у каждой своя кнопка и пауза.",
    loc_keyboard_note: "Сейчас действие — нажатие клавиши, место клика не используется.",
    pick_on_screen: "Выбрать на экране",
    select_on_screen: "Выделить на экране",
    width: "Ширина",
    height: "Высота",
    add_on_screen: "Добавить на экране",
    add_at_cursor: "Под курсором",
    in_order: "По порядку",
    random: "Случайно",
    clear: "Очистить",
    route_empty: "Точек пока нет. Добавьте их на экране или наведите курсор на цель и нажмите горячую клавишу.",
    pause: "пауза",
    tip_up: "Выше",
    tip_down: "Ниже",
    tip_remove: "Удалить",
    return_cursor: "Возвращать курсор",
    return_cursor_hint: "После каждого клика курсор вернётся на место",

    hotkeys: "Горячие клавиши",
    hotkeys_hint: "Работают глобально, даже когда clkr свёрнут.",
    hk_toggle: "Старт / стоп",
    hk_mode: "Режим",
    hk_mode_toggle: "Нажатие",
    hk_mode_hold: "Удержание",
    hk_hold_hint: "Кликает, только пока клавиша зажата",
    hk_add_point: "Точка маршрута",
    safety: "Запуск и безопасность",
    start_delay: "Задержка старта",
    start_delay_hint: "Время, чтобы навести курсор",
    failsafe: "Стоп при движении мыши",
    failsafe_hint: "В режимах точки, зоны и маршрута",
    self_click_note: "clkr не кликает по своему окну: пока курсор над ним, клики на паузе.",
    interface: "Интерфейс",
    language: "Язык",
    theme: "Тема",
    theme_dark: "Тёмная",
    theme_light: "Светлая",
    always_on_top: "Поверх всех окон",
    statistics: "Статистика",
    total_clicks: "Всего кликов",
    reset: "Сбросить",
    config_at: "Файл настроек",

    stopped_limit: "Готово: лимит достигнут",
    stopped_mouse: "Остановлено: мышь сдвинута",
    point_added: "Точка маршрута добавлена",
    points_added: "Добавлено в маршрут:",

    pick_hint_point: "Кликните, чтобы выбрать точку   ·   стрелки — точнее   ·   Esc — отмена",
    pick_hint_area: "Зажмите и протяните, чтобы выделить область   ·   Esc — отмена",
    pick_hint_points: "Кликайте точки по порядку   ·   Backspace — отменить   ·   Enter или ПКМ — готово",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_plurals() {
        let forms: Vec<_> = [0, 1, 2, 5, 11, 12, 21, 22, 25, 101, 111].iter().map(|&n| RU.clicks_word(n)).collect();
        assert_eq!(
            forms,
            ["кликов", "клик", "клика", "кликов", "кликов", "кликов", "клик", "клика", "кликов", "клик", "кликов"]
        );
        assert_eq!(EN.clicks_word(1), "click");
        assert_eq!(EN.clicks_word(3), "clicks");
    }
}
