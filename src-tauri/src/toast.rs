// The small notices in the bottom-right corner of the screen: a calendar
// event due, and a new letter that carries a verification code.
//
// They are a window of their own, laid over the work area's bottom-right
// corner (above Windows' taskbar), always on top, never taking the focus
// and click-through: what is underneath stays usable while one shows. The
// page queues them (toast.js) and, once the queue is empty, asks this side
// to hide the window until the next one.
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
const WIDTH: f64 = 400.0;
const HEIGHT: f64 = 160.0;
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

// A calendar event is due: its date (and time, when it has one) and what
// it is. The page writes the date in the person's language and shape.
pub fn calendar(sh: &Arc<Shared>, date: &str, start: &str, title: &str) {
    show(
        sh,
        json!({
            "kind": "calendar",
            "date": date,
            "start": start,
            "allDay": start.trim().is_empty(),
            "title": title,
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
    let screens = crate::screen::read(win.app_handle());
    let Some(area) = screens.primary.or_else(|| screens.areas.first().copied()) else { return };
    let width = (WIDTH * area.sf).round() as u32;
    let height = (HEIGHT * area.sf).round() as u32;
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
