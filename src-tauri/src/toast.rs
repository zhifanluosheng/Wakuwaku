// The small notices in the bottom-right corner of the screen: a calendar
// event due, and a new letter that carries a verification code.
//
// They are a window of their own, laid over the work area's bottom-right
// corner (above Windows' taskbar), always on top and never taking the focus.
// A calendar notice stays click-through; a code notice takes the mouse so
// its code can be copied, and the page asks for that only while it shows.
// The page queues them (toast.js) and, once the queue is empty, asks this
// side to hide the window until the next one.
//
// The chime is a short WAV kept in the exe; Windows plays it through
// winmm's PlaySound. There is no setting for it yet: the person asked for
// the sound now, and a switch can come later without moving any of this.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::Shared;

const LABEL: &str = "toast";
const WIDTH: f64 = 420.0;
// The page measures its card and asks for the height it needs; this is the
// height the window is made at, and the first one before that measurement.
const HEIGHT: f64 = 210.0;
const MIN_HEIGHT: f64 = 180.0;
const MAX_HEIGHT: f64 = 320.0;
const MARGIN: f64 = 16.0;

static WINDOW: Mutex<Option<WebviewWindow>> = Mutex::new(None);
static PENDING: Mutex<Vec<Value>> = Mutex::new(Vec::new());
static READY: AtomicBool = AtomicBool::new(false);
static BUILDING: AtomicBool = AtomicBool::new(false);
// Which notice is the newest: a page that emptied its queue asks the window
// to hide, and only the newest one may do it (a notice that arrived in the
// meantime has already shown the window again).
static SEQ: AtomicU64 = AtomicU64::new(0);
// The window could not be made, or its page never came up: the system's
// own toast carries the rest (it has no avatar, but the words arrive).
static BROKEN: AtomicBool = AtomicBool::new(false);

// A calendar event is due: what it is, when, where, and how it was set to
// remind. The page writes the date in the person's language and shape.
#[allow(clippy::too_many_arguments)]
pub fn calendar(
    sh: &Arc<Shared>,
    date: &str,
    start: &str,
    end: &str,
    title: &str,
    location: &str,
    repeat: &str,
    remind_days: Option<u64>,
    remind_minutes: Option<u64>,
) {
    show(
        sh,
        json!({
            "kind": "calendar",
            "date": date,
            "start": start,
            "end": end,
            "allDay": start.trim().is_empty(),
            "title": title,
            "where": location,
            "repeat": repeat,
            "remindDays": remind_days,
            "remind": remind_minutes,
            "lang": sh.lang(),
        }),
    );
}

// A new letter with a code in it. The page draws the sender's initial as
// an avatar, the way the Mail page does.
pub fn mail_code(sh: &Arc<Shared>, name: &str, address: &str, code: &str) {
    let name = name.trim();
    let address = address.trim();
    show(
        sh,
        json!({
            "kind": "mail",
            "name": if name.is_empty() { address } else { name },
            "address": address,
            "code": code,
            "lang": sh.lang(),
        }),
    );
}

fn show(sh: &Arc<Shared>, payload: Value) {
    play_sound();
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let payload = match payload {
        Value::Object(mut map) => {
            map.insert("seq".into(), json!(seq));
            Value::Object(map)
        }
        other => other,
    };
    if BROKEN.load(Ordering::SeqCst) {
        let _ = crate::notify::show(&plain(&payload));
        return;
    }
    PENDING.lock().unwrap().push(payload);
    init(sh);
    flush();
}

// The window is made once, at startup, hidden: the first notice is then
// only an event away, not a page load too. If something goes wrong the
// system's toast still says what happened.
pub fn init(sh: &Arc<Shared>) {
    if WINDOW.lock().unwrap().is_some() || BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let sh = sh.clone();
    crate::spawn_named("toast-window", move || {
        let built = WebviewWindowBuilder::new(&sh.app, LABEL, WebviewUrl::App("pet/toast.html".into()))
            .title("wakuwaku toast")
            .inner_size(WIDTH, HEIGHT)
            .additional_browser_args(crate::browser_args(&sh))
            .data_directory(crate::webview_dir(&sh))
            .transparent(true)
            .decorations(false)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .skip_taskbar(true)
            .shadow(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .focusable(false)
            .focused(false)
            .visible(false)
            .build();
        match built {
            Ok(win) => {
                place(&win);
                let _ = win.set_ignore_cursor_events(true);
                #[cfg(windows)]
                if let Ok(hwnd) = win.hwnd() {
                    crate::tasks::no_transitions(hwnd.0 as isize);
                }
                sh.own_window(&win);
                *WINDOW.lock().unwrap() = Some(win);
                flush();
                // A window whose page never speaks: after ten seconds the
                // system's toast takes over, so a notice is not lost.
                crate::spawn_named("toast-watchdog", move || {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    if !READY.load(Ordering::SeqCst) && !BROKEN.swap(true, Ordering::SeqCst) {
                        fallback_all();
                    }
                });
            }
            Err(err) => {
                sh.log(&format!("toast: could not make its window: {err}"));
                BROKEN.store(true, Ordering::SeqCst);
                fallback_all();
            }
        }
        BUILDING.store(false, Ordering::SeqCst);
    });
}

// Its page is up and listening.
#[tauri::command]
pub fn toast_ready(window: WebviewWindow) {
    if window.label() != LABEL {
        return;
    }
    *WINDOW.lock().unwrap() = Some(window);
    READY.store(true, Ordering::SeqCst);
    flush();
}

// Its queue is empty: out of the way until the next one. The sequence keeps
// a notice that arrived in the meantime from being hidden with it.
#[tauri::command]
pub fn toast_hide(window: WebviewWindow, seq: u64) {
    if window.label() == LABEL && SEQ.load(Ordering::SeqCst) == seq {
        let _ = window.hide();
    }
}

// Only a code notice wants the mouse (to copy its code). The page calls this
// as each notice is drawn, so the next calendar notice goes back to passing
// clicks through the window.
#[tauri::command]
pub fn toast_interactive(window: WebviewWindow, on: bool) {
    if window.label() == LABEL {
        let _ = window.set_ignore_cursor_events(!on);
    }
}

// The page has measured its card: the window takes just that height and
// stays at the work area's bottom-right corner.
#[tauri::command]
pub fn toast_fit(window: WebviewWindow, height: f64) {
    if window.label() == LABEL && height.is_finite() {
        place_height(&window, height.clamp(MIN_HEIGHT, MAX_HEIGHT));
    }
}

// The page's own clipboard call can be refused in a window that never takes
// the focus; Windows' clipboard is set directly then. Elsewhere the page
// falls back to its document copy.
#[tauri::command]
pub fn toast_copy(window: WebviewWindow, text: String) -> bool {
    if window.label() != LABEL {
        return false;
    }
    #[cfg(windows)]
    {
        copy_text(&text)
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        false
    }
}

#[cfg(windows)]
fn copy_text(text: &str) -> bool {
    use std::ffi::c_void;
    use std::time::Duration;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(hwnd: *mut c_void) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
        fn CloseClipboard() -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
        fn GlobalLock(mem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(mem: *mut c_void) -> i32;
        fn GlobalFree(mem: *mut c_void) -> *mut c_void;
    }

    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();

    // Another program may hold the clipboard for a moment.
    let mut opened = false;
    for _ in 0..8 {
        // SAFETY: no window owns this clipboard session; it is closed below.
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            opened = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    if !opened {
        return false;
    }

    let mut copied = false;
    // SAFETY: the handle is moveable, filled while locked, and then either
    // handed to the clipboard (which owns it) or freed here.
    unsafe {
        let _ = EmptyClipboard();
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if handle.is_null() {
            let _ = CloseClipboard();
            return false;
        }
        let at = GlobalLock(handle) as *mut u16;
        if at.is_null() {
            let _ = GlobalFree(handle);
            let _ = CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), at, wide.len());
        let _ = GlobalUnlock(handle);
        if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
            let _ = GlobalFree(handle);
        } else {
            copied = true;
        }
        let _ = CloseClipboard();
    }
    copied
}

fn flush() {
    if !READY.load(Ordering::SeqCst) {
        return;
    }
    let Some(win) = WINDOW.lock().unwrap().clone() else { return };
    let payloads: Vec<Value> = {
        let mut pending = PENDING.lock().unwrap();
        pending.drain(..).collect()
    };
    if payloads.is_empty() {
        return;
    }
    place(&win);
    let _ = win.show();
    for payload in payloads {
        let _ = win.emit("toast:show", payload);
    }
}

// The work area's bottom-right corner, clear of the taskbar, in physical
// pixels: the monitor the island is on (the main one), as Windows gives it.
fn place(win: &WebviewWindow) {
    place_height(win, HEIGHT);
}

fn place_height(win: &WebviewWindow, height: f64) {
    let screens = crate::screen::read(win.app_handle());
    let Some(area) = screens.primary.or_else(|| screens.areas.first().copied()) else { return };
    let width = (WIDTH * area.sf).round() as u32;
    let height = (height * area.sf).round() as u32;
    let margin = (MARGIN * area.sf).round() as i32;
    let x = area.x + area.w - width as i32 - margin;
    let y = area.y + area.h - height as i32 - margin;
    let _ = win.set_size(PhysicalSize::new(width.max(1), height.max(1)));
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn plain(payload: &Value) -> String {
    match payload["kind"].as_str() {
        Some("calendar") => {
            let date = payload["date"].as_str().unwrap_or("");
            let start = payload["start"].as_str().unwrap_or("");
            let title = payload["title"].as_str().unwrap_or("");
            if start.is_empty() {
                format!("{date} · {title}")
            } else {
                format!("{date} {start} · {title}")
            }
        }
        Some("mail") => {
            let name = payload["name"].as_str().unwrap_or("");
            let address = payload["address"].as_str().unwrap_or("");
            let code = payload["code"].as_str().unwrap_or("");
            format!("{name} <{address}> · {code}")
        }
        _ => String::new(),
    }
}

fn fallback_all() {
    let payloads: Vec<Value> = {
        let mut pending = PENDING.lock().unwrap();
        pending.drain(..).collect()
    };
    for payload in payloads {
        let words = plain(&payload);
        if !words.is_empty() {
            let _ = crate::notify::show(&words);
        }
    }
}

// A short two-note chime, played asynchronously so the reminder does not
// wait on it. The bytes stay in the exe for as long as Windows may read
// them (SND_MEMORY).
#[cfg(windows)]
fn play_sound() {
    use std::ffi::c_void;

    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(sound: *const u16, module: *mut c_void, flags: u32) -> i32;
    }

    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    static CHIME: &[u8] = include_bytes!("../sounds/notify.wav");
    // SAFETY: CHIME is a static WAV and stays valid; Windows only reads it.
    unsafe {
        PlaySoundW(CHIME.as_ptr() as *const u16, std::ptr::null_mut(), SND_ASYNC | SND_NODEFAULT | SND_MEMORY);
    }
}

#[cfg(not(windows))]
fn play_sound() {}
