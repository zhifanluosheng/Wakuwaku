// The settings, grown out of the island (src/pet/settings.js): what they
// show (a snapshot, pushed again as things change) and what they change.
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::{json, Map, Value};
use tauri::{AppHandle, WebviewWindow};

use crate::proxy::{Kind, Proxy};
use crate::{connection, data, fetch, fullscreen, i18n, island, mail, now_ms, pet, scripts, set_display, set_out, shared, Shared};

const REPO: &str = "https://github.com/zhoupengjie/wakuwaku";

// Each agent's own page, by its id (src/pet/agents.js), for the Agent page's
// other supported tools: the only pages it may open.
const AGENT_SITES: [(&str, &str); 9] = [
    ("claude", "https://claude.com/product/claude-code"),
    ("codex", "https://developers.openai.com/codex"),
    ("agy", "https://antigravity.google/"),
    ("opencode", "https://opencode.ai/"),
    ("omp", "https://github.com/can1357/oh-my-pi"),
    ("zcode", "https://zcode.z.ai/"),
    ("trae", "https://www.trae.cn/"),
    ("workbuddy", "https://www.workbuddy.ai/"),
    ("dsh", "https://github.com/deepseek-ai/deepseek-harness"),
];

// Everything the settings show.
pub fn snapshot(sh: &Shared) -> Value {
    let pets: Vec<Value> = data::pets(&sh.dir)
        .iter()
        .map(|p| json!({ "id": p.id, "name": p.name, "author": p.author, "url": data::asset_url(&p.sheet), "version": p.version, "codex": p.from_codex }))
        .collect();
    let (now, sessions) = {
        let pet = sh.pet.lock().unwrap();
        (pet.get(now_ms()), pet.list())
    };
    let (win_light, win_accent) = crate::wallpaper::look();
    // Whether either is connected, as the island tells it (connection.rs).
    let conn = connection::connection(sh.port);
    let omp = connection::omp_hooks_status(sh.port);
    if connection::note_connected(conn != "none" || connection::is_codex_connected() || connection::is_agy_connected() || connection::is_opencode_connected(sh.port) || matches!(omp, "ok" | "stale" | "partial")) {
        sh.redraw();
    }
    let omp_files = connection::omp_files();
    // The monitor's source in effect: the one chosen, else LHM while it answers.
    let lhm = crate::lhm::view();
    let lhm_status = lhm["status"].as_str().unwrap_or("").to_string();
    let monitor_source = crate::hw::Source::of(&sh.setting("monitor"), &sh.setting("monitorSource"), lhm_status == "found");
    let lhm_shown = monitor_source == crate::hw::Source::Lhm && sh.island.lock().unwrap().settings_open;
    json!({
        "settings": Value::Object(sh.settings.lock().unwrap().clone()),
        "lang": sh.lang(),
        "pets": pets,
        "now": now,
        "sessions": sessions,
        "asks": sh.asks.lock().unwrap().views(),
        "isVisible": sh.is_visible(),
        "hidden": sh.hidden.load(Ordering::SeqCst),
        "connection": conn,
        "pluginCommands": connection::PLUGIN_COMMANDS,
        "hooks": connection::hooks_status(sh.port),
        // Whether any Claude Code event came since she started: installed,
        // and a session seen (connected), or still waiting for one.
        "claudeSeen": sh.claude_seen.load(Ordering::SeqCst) > 0,
        "settingsFile": connection::claude_settings_file().to_string_lossy(),
        // The same as read (~ for the home folder), whether a copy of it was
        // kept (none: there was no file before), and whether Claude Code is
        // on this computer at all (its folder): the welcome offers the ones
        // that are.
        "settingsShown": connection::shown_path(&connection::claude_settings_file()),
        "settingsBackup": connection::backup_of(&connection::claude_settings_file()).exists(),
        "claudeHere": connection::claude_here(),
        // Codex's hooks.json, whether any Codex event came (they run once
        // trusted), and its version: one before 0.148 gets no background hooks.
        "codex": {
            "hooks": connection::codex_hooks_status(),
            "file": connection::codex_hooks_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::codex_hooks_file()),
            "seen": sh.codex_seen.load(Ordering::SeqCst) > 0,
            "version": connection::codex_version().map(|(a, b, c)| format!("{a}.{b}.{c}")),
            "async": connection::codex_runs_async(),
        },
        // Antigravity's hooks.json (connection.rs), the same, and whether it
        // is on this computer at all (agy's folder, or its 2.0 app's).
        "agy": {
            "hooks": connection::agy_hooks_status(),
            "file": connection::agy_hooks_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::agy_hooks_file()),
            "backup": connection::backup_of(&connection::agy_hooks_file()).exists(),
            "seen": sh.agy_seen.load(Ordering::SeqCst) > 0,
        },
        // Our plugin in OpenCode's plugin folder (connection.rs): absent (no
        // OpenCode here), missing, ok, stale (another port or version), and
        // whether any of its events came since she started.
        "opencode": {
            "plugin": connection::opencode_status(sh.port),
            "file": connection::opencode_plugin_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::opencode_plugin_file()),
            "seen": sh.opencode_seen.load(Ordering::SeqCst) > 0,
        },
        // omp's extension (connection.rs): in its agent folder, and each
        // profile's (more: how many besides the first); whether any omp
        // event came; whether omp is on this computer at all.
        "omp": {
            "hooks": omp,
            "file": omp_files.first().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
            "fileShown": omp_files.first().map(|f| connection::shown_path(f)).unwrap_or_default(),
            "more": omp_files.len().saturating_sub(1),
            "seen": sh.omp_seen.load(Ordering::SeqCst) > 0,
        },
        // ZCode's config.json (connection.rs), the same; absent without its folder.
        "zcode": {
            "hooks": connection::zcode_hooks_status(),
            "file": connection::zcode_config_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::zcode_config_file()),
            "backup": connection::backup_of(&connection::zcode_config_file()).exists(),
            "seen": sh.zcode_seen.load(Ordering::SeqCst) > 0,
        },
        // Trae CN's hooks.json (connection.rs), Claude Code's shape; absent
        // without its folder.
        "trae": {
            "hooks": connection::trae_hooks_status(),
            "file": connection::trae_hooks_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::trae_hooks_file()),
            "backup": connection::backup_of(&connection::trae_hooks_file()).exists(),
            "seen": sh.trae_seen.load(Ordering::SeqCst) > 0,
        },
        // WorkBuddy's settings.json (Claude Code's hooks); WorkBuddy AI's
        // folder first, the legacy one beside it.
        "workbuddy": {
            "hooks": connection::workbuddy_hooks_status(),
            "file": connection::workbuddy_settings_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::workbuddy_settings_file()),
            "backup": connection::backup_of(&connection::workbuddy_settings_file()).exists(),
            "seen": sh.workbuddy_seen.load(Ordering::SeqCst) > 0,
        },
        // DeepSeek Harness's home-level patch file, where its hook bridge is
        // inserted (connection.rs), the same; absent without its folder.
        "dsh": {
            "hooks": connection::dsh_hooks_status(),
            "file": connection::dsh_patch_file().to_string_lossy(),
            "fileShown": connection::shown_path(&connection::dsh_patch_file()),
            "backup": connection::backup_of(&connection::dsh_patch_file()).exists(),
            // The bridge's hook list, in her data folder (the Agent page shows both).
            "listShown": connection::shown_path(&connection::dsh_hooks_file()),
            "seen": sh.dsh_seen.load(Ordering::SeqCst) > 0,
        },
        // The agents' versions where a file of theirs says (connection.rs
        // versions; Codex's is its block's): the Agent page shows them.
        "agentVersions": connection::versions(),
        // How many displays there are: her taskbar on all of them is offered
        // only with more than one (Look, taskbarDisplays).
        "displayCount": sh.screens().areas.len(),
        "loginAtStart": connection::is_open_at_login(),
        // Windows' own mode and accent colour (settings_win_look sets them).
        "winLook": { "light": win_light, "accent": crate::theme::current_accent().or(win_accent.map(|a| a.base)) },
        // The desktop's picture as it is (paper.rs).
        "winPaper": crate::paper::now(),
        "version": sh.app.package_info().version.to_string(),
        // Her updates: this version, the latest, an update under way (updates.rs).
        "update": crate::updates::view(&sh),
        // Her crash reports: as set, those waiting, the last sent (crash.rs).
        "crash": crate::crash::view(sh),
        "fullscreenAvailable": fullscreen::AVAILABLE,
        // Whether Windows' acrylic can go under the open island (backplate.rs).
        "plate": crate::backplate::available(),
        "widgets": sh.widgets_view(),
        // The monitor's hardware sensors: LibreHardwareMonitor found or
        // not, and its sensors (lhm.rs); the built-in ones (hw.rs); which
        // of them it shows: the one chosen, else LHM while it answers.
        "lhm": lhm,
        "hw": crate::hw::view(),
        "monitorSource": monitor_source.name(),
        // The CPU temperature/power helper's state (sensors.rs, #18 part B):
        // off / pawnioMissing / settingUp / running / notAnswering / needsUpdate.
        "cpuSensors": crate::sensors::view(&sh.dir),
        // LibreHardwareMonitor got, set up, started (lhm_setup.rs): what is
        // offered, what was found, a download under way; looked at again
        // while the settings show it.
        "lhmSetup": crate::lhm_setup::view(sh, &lhm_status, lhm_shown),
        // The plugins she runs, and where waku is for the terminal.
        "plugins": scripts::view(sh),
        // The mail accounts, each with how its watch is going.
        "mail": mail::view(sh),
        // Who a letter can go to (claude, codex, agy: found or not; looked
        // for again while the settings are open), and the letters being
        // read in the background or read this time.
        "mailAgents": mail::agent::available(),
        // The models each can be asked for, with the efforts they take.
        "mailModels": mail::agent::models(sh),
        "mailTalks": mail::agent::talks(sh),
        // Windows' proxy as mail would take it (the Mail page's "follow the system").
        "mailSystemProxy": mail::proxy::system_view(),
        // The mail kept changed: the Mail page asks for its list again.
        "mailRev": mail::rev(),
        "waku": scripts::waku_path(sh).to_string_lossy(),
        // Her own exe, for the MCP server's command (the MCP page).
        "exe": crate::data::exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        "port": sh.port,
    })
}

// One mode's island look: its ground (#rrggbb), how solid (20 to 100 %),
// how blurred what is behind (0 to 60 px), its edge, no capsule collapsed;
// each part may be left out (its default), nothing else.
fn is_island_look(v: &Value) -> bool {
    v.is_null()
        || v.as_object().is_some_and(|m| {
            m.iter().all(|(k, v)| match k.as_str() {
                "bg" => v.as_str().is_some_and(|s| s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())),
                "op" => v.as_u64().is_some_and(|n| (20..=100).contains(&n)),
                "blur" => v.as_u64().is_some_and(|n| n <= 60),
                "edge" => matches!(v.as_str(), Some("line" | "ring" | "none")),
                "bare" => v.is_boolean(),
                _ => false,
            })
        })
}

// The taskbar's layout: { left, center, right }, each a list of its parts
// (island.js TB_PARTS), none twice and nothing else; one left out goes where
// the default has it.
const TASKBAR_PARTS: [&str; 8] = ["her", "start", "apps", "monitor", "tray", "keys", "quick", "clock"];

// A widget put on the taskbar (widgets.js isEntry): { id, type, of, size,
// name, icon }, its id "w" and up to 15 of a-z 0-9; a widget's or a
// plugin's (of: its id), or a spacer (flex, gap, sep); or several in one
// place (carousel: in turn; stack: the first, the rest opened over it),
// { id, type, items }, 2 to 8 of them, each { type, of, name, icon } as a
// widget's or a plugin's are, none twice.
fn is_taskbar_widget(v: &Value) -> Option<&str> {
    let w = v.as_object()?;
    let id = w.get("id")?.as_str()?;
    let ok_id = id.len() >= 2 && id.len() <= 16 && id.starts_with('w') && id[1..].chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let kind = w.get("type")?.as_str()?;
    let of = w.get("of").map(|of| of.as_str().filter(|of| crate::widgets::is_id(of)));
    let ok_of = match kind {
        "widget" | "plugin" => of.is_some_and(|of| of.is_some_and(|of| is_source_of(kind, of))),
        "flex" | "gap" | "sep" | "carousel" | "stack" => of.is_none(),
        _ => false,
    };
    let group = matches!(kind, "carousel" | "stack");
    let ok_items = match w.get("items").and_then(Value::as_array) {
        Some(items) if group => (2..=8).contains(&items.len()) && items.iter().all(is_member) && items.iter().enumerate().all(|(i, m)| !items[..i].iter().any(|x| x["type"] == m["type"] && x["of"] == m["of"])),
        None => !group,
        _ => false,
    };
    let ok_rest = w.iter().all(|(key, v)| match key.as_str() {
        "id" | "type" | "of" | "items" => true,
        "size" => !group && matches!(v.as_str(), Some("S" | "M")),
        "name" => v.as_str().is_some_and(is_name),
        "icon" => v.as_str().is_some_and(|i| crate::widgets::ICONS.contains(&i)),
        _ => false,
    });
    (ok_id && ok_of && ok_items && ok_rest).then_some(id)
}

fn is_name(n: &str) -> bool {
    n.chars().count() <= 40 && !n.chars().any(char::is_control)
}

// A widget by its id, or a plugin she runs.
fn is_source_of(kind: &str, of: &str) -> bool {
    match kind {
        "widget" => crate::widgets::is_id(of),
        "plugin" => crate::scripts::SCRIPTS.iter().any(|s| s.id == of),
        _ => false,
    }
}

// One of several in one place: a widget's or a plugin's, as put there alone.
fn is_member(v: &Value) -> bool {
    v.as_object().is_some_and(|m| {
        let kind = m.get("type").and_then(Value::as_str).unwrap_or("");
        m.get("of").and_then(Value::as_str).is_some_and(|of| is_source_of(kind, of))
            && m.iter().all(|(key, v)| match key.as_str() {
                "type" | "of" => true,
                "name" => v.as_str().is_some_and(is_name),
                "icon" => v.as_str().is_some_and(|i| crate::widgets::ICONS.contains(&i)),
                _ => false,
            })
    })
}

// Whether a taskbar layout shows this widget: put there by itself, or its
// plugin's; alone or with others in one place.
pub fn is_on_taskbar(layout: &Value, widget: &str) -> bool {
    let owner = crate::scripts::owner_of(widget);
    let shows = |e: &Value| match (e["type"].as_str(), e["of"].as_str()) {
        (Some("widget"), Some(of)) => of == widget,
        (Some("plugin"), Some(of)) => Some(of) == owner,
        _ => false,
    };
    layout
        .as_object()
        .into_iter()
        .flat_map(|runs| runs.values())
        .filter_map(Value::as_array)
        .flatten()
        .any(|e| shows(e) || e["items"].as_array().is_some_and(|items| items.iter().any(shows)))
}

// The desktop's widgets (deskgrid.js isEntry, isLayout): up to 32, each
// { id, type, of, size, name, icon, d, at } alone (a widget's or a
// plugin's) or { id, type: "stack", items, size, d, at } several in one, 2
// to 8 as the taskbar's groups have them, or { id, type: "board", name,
// items, folded, d, at } a board (its name 16 characters at most, its tiles
// is_board_tiles); size s, m or l; d the display it
// is on (desk.rs key_of: a-z 0-9), at where it stood on each display it has
// been on, 8 at most, { key: [column, row] }. Each id once, each widget or
// plugin once.
fn is_desk_entry(v: &Value) -> bool {
    let Some(e) = v.as_object() else { return false };
    let Some(id) = e.get("id").and_then(Value::as_str) else { return false };
    let ok_id = id.len() >= 2 && id.len() <= 16 && id.starts_with('w') && id[1..].chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let is_key = |k: &str| !k.is_empty() && k.len() <= 40 && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let is_cell = |v: &Value| v.as_array().is_some_and(|a| a.len() == 2 && a.iter().all(|n| n.as_u64().is_some_and(|n| n <= 255)));
    if e.get("type").and_then(Value::as_str) == Some("board") {
        let ok_rest = e.iter().all(|(key, v)| match key.as_str() {
            "id" | "type" | "items" => true,
            "name" => v.as_str().is_some_and(|n| n.chars().count() <= 16 && !n.chars().any(char::is_control)),
            "folded" => v.is_boolean(),
            "d" => v.as_str().is_some_and(is_key),
            "at" => v.as_object().is_some_and(|at| at.len() <= 8 && at.iter().all(|(k, c)| is_key(k) && is_cell(c))),
            _ => false,
        });
        return ok_id && ok_rest && e.get("items").is_some_and(is_board_tiles);
    }
    let stack = e.get("type").and_then(Value::as_str) == Some("stack");
    let ok_kind = if stack {
        e.get("of").is_none()
            && e.get("items").and_then(Value::as_array).is_some_and(|items| (2..=8).contains(&items.len()) && items.iter().all(is_member) && items.iter().enumerate().all(|(i, m)| !items[..i].iter().any(|x| x["type"] == m["type"] && x["of"] == m["of"])))
    } else {
        e.get("items").is_none() && is_member(&json!({ "type": e.get("type"), "of": e.get("of") }))
    };
    let ok_rest = e.iter().all(|(key, v)| match key.as_str() {
        "id" | "type" | "of" | "items" => true,
        "size" => matches!(v.as_str(), Some("s" | "m" | "l")),
        "name" => !stack && v.as_str().is_some_and(is_name),
        "icon" => !stack && v.as_str().is_some_and(|i| crate::widgets::ICONS.contains(&i)),
        "d" => v.as_str().is_some_and(is_key),
        "at" => v.as_object().is_some_and(|at| at.len() <= 8 && at.iter().all(|(k, c)| is_key(k) && is_cell(c))),
        _ => false,
    });
    ok_id && e.contains_key("size") && ok_kind && ok_rest
}

// A board's tiles (deskgrid.js isBoardEntry): 16 at most, each a member
// with its size and its cell on the board's grid of 8 columns and 4 rows,
// whole on it, none over another, each widget or plugin once.
fn is_board_tiles(v: &Value) -> bool {
    let Some(tiles) = v.as_array() else { return false };
    let span = |s: &str| match s {
        "s" => Some((2, 2)),
        "m" => Some((4, 2)),
        "l" => Some((4, 4)),
        _ => None,
    };
    let mut placed: Vec<(u64, u64, u64, u64)> = Vec::new();
    let mut members: Vec<(&Value, &Value)> = Vec::new();
    for t in tiles {
        let Some(o) = t.as_object() else { return false };
        let Some((w, h)) = o.get("size").and_then(Value::as_str).and_then(span) else { return false };
        let Some([c, r]) = o.get("at").and_then(Value::as_array).and_then(|a| <[Value; 2]>::try_from(a.clone()).ok()) else { return false };
        let (Some(c), Some(r)) = (c.as_u64(), r.as_u64()) else { return false };
        let member: serde_json::Map<String, Value> = o.iter().filter(|(k, _)| !matches!(k.as_str(), "size" | "at")).map(|(k, v)| (k.clone(), v.clone())).collect();
        if c + w > 8 || r + h > 4 || !is_member(&Value::Object(member)) {
            return false;
        }
        if placed.iter().any(|&(pc, pr, pw, ph)| c < pc + pw && pc < c + w && r < pr + ph && pr < r + h) {
            return false;
        }
        let key = (&o["type"], &o["of"]);
        if members.contains(&key) {
            return false;
        }
        placed.push((c, r, w, h));
        members.push(key);
    }
    tiles.len() <= 16
}

pub fn is_desk_layout(v: &Value) -> bool {
    let Some(entries) = v.as_array() else { return false };
    if entries.len() > 32 || !entries.iter().all(is_desk_entry) {
        return false;
    }
    let ids: Vec<&Value> = entries.iter().map(|e| &e["id"]).collect();
    let members: Vec<(Value, Value)> = entries
        .iter()
        .flat_map(|e| match e["items"].as_array() {
            Some(items) => items.iter().map(|m| (m["type"].clone(), m["of"].clone())).collect::<Vec<_>>(),
            None => vec![(e["type"].clone(), e["of"].clone())],
        })
        .collect();
    ids.iter().enumerate().all(|(i, id)| !ids[..i].contains(id)) && members.iter().enumerate().all(|(i, m)| !members[..i].contains(m))
}

// Boards kept as combos (the picker's 组合, a board's 存为组合…), checked as
// deskgrid.js isBoardSets checks them: up to 20, each { id, name, items,
// saved } and nothing else, ids and names each once. id: "s" and 1 to 15 of
// a-z 0-9; name: 1 to 16 characters, none a control character, no space at
// either end (as JavaScript's trim() has them); items: 1 to 16 tiles {
// type, of, size, at } as a board's (is_board_tiles), with no names or
// icons (those are the widgets' own, read as it goes up again); saved: when,
// ms since 1970 (a whole number JavaScript holds exactly).
pub fn is_board_sets(v: &Value) -> bool {
    let Some(list) = v.as_array() else { return false };
    if list.len() > 20 || !list.iter().all(is_board_set) {
        return false;
    }
    list.iter().enumerate().all(|(i, s)| !list[..i].iter().any(|o| o["id"] == s["id"] || o["name"] == s["name"]))
}

fn is_board_set(v: &Value) -> bool {
    const MAX_SAFE: u64 = (1 << 53) - 1;
    let Some(s) = v.as_object() else { return false };
    let id = s.get("id").and_then(Value::as_str).is_some_and(|id| (2..=16).contains(&id.len()) && id.starts_with('s') && id[1..].bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    let name = s.get("name").and_then(Value::as_str).is_some_and(is_set_name);
    let saved = s.get("saved").and_then(Value::as_u64).is_some_and(|ms| ms <= MAX_SAFE);
    let tiles = s.get("items").and_then(Value::as_array).is_some_and(|tiles| !tiles.is_empty() && tiles.iter().all(|t| t.as_object().is_some_and(|t| t.keys().all(|k| matches!(k.as_str(), "type" | "of" | "size" | "at")))));
    let keys = s.keys().all(|k| matches!(k.as_str(), "id" | "name" | "items" | "saved"));
    id && name && saved && tiles && keys && s.get("items").is_some_and(is_board_tiles)
}

// A combo's name (deskgrid.js isSetName): a board's (16 characters at most,
// no control character: U+0000 to U+001F, U+007F), not blank, and with no
// space at either end, the spaces being those JavaScript's trim() takes off
// (Rust's White_Space less U+0085, and U+FEFF).
fn is_set_name(n: &str) -> bool {
    let space = |c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}';
    !n.is_empty() && n.chars().count() <= 16 && !n.chars().any(|c| c <= '\u{1f}' || c == '\u{7f}') && n.trim_matches(space) == n
}

// Her capsule as a container of widgets (capsule.js: storedOf and the
// patches it writes): { members, spin }, each of them optional (a turn's
// length is kept on its own before any member is ordered: capsule.js
// spinPatch). members: up to 64 of { key, mode } and nothing else, each key
// once; key "widget:" or "plugin:" and 1 to 40 of a-z 0-9 _ -; mode turn
// (takes its turn in the closed capsule, 轮播), pin (always at its right end,
// 固定) or open (only in the open island, 展开时). spin: the seconds a turn
// lasts, as widgetSpin (0, 5, 8, 15). The order and the monitor's pin are
// written to widgetOrder and monitor.pin as well, which the widgets read.
pub fn is_capsule(v: &Value) -> bool {
    let Some(c) = v.as_object() else { return false };
    c.iter().all(|(key, v)| match key.as_str() {
        "members" => v.as_array().is_some_and(|list| list.len() <= 64 && list.iter().all(is_capsule_member) && list.iter().enumerate().all(|(i, m)| !list[..i].iter().any(|o| o["key"] == m["key"]))),
        "spin" => matches!(v.as_u64(), Some(0 | 5 | 8 | 15)),
        _ => false,
    })
}

fn is_capsule_member(v: &Value) -> bool {
    let Some(m) = v.as_object() else { return false };
    let key = m.get("key").and_then(Value::as_str).and_then(|k| k.strip_prefix("widget:").or_else(|| k.strip_prefix("plugin:")));
    let key = key.is_some_and(|id| (1..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'));
    let mode = matches!(m.get("mode").and_then(Value::as_str), Some("turn" | "pin" | "open"));
    key && mode && m.len() == 2
}

// Whether the desktop shows this widget: put there by itself, or its
// plugin's; alone or in a stack.
pub fn is_on_desk(layout: &Value, widget: &str) -> bool {
    let owner = crate::scripts::owner_of(widget);
    let shows = |e: &Value| match (e["type"].as_str(), e["of"].as_str()) {
        (Some("widget"), Some(of)) => of == widget,
        (Some("plugin"), Some(of)) => Some(of) == owner,
        _ => false,
    };
    layout.as_array().into_iter().flatten().any(|e| shows(e) || e["items"].as_array().is_some_and(|items| items.iter().any(shows)))
}

// Put elsewhere than the island: on the desktop, or on the taskbar while
// that is her home. A widget is in one place (the widget design's §7): it
// neither takes turns in the closed island nor stays pinned at its end
// there (island.js placedOut); the open island lists it all the same.
pub fn is_placed_out(desk: &Value, taskbar: &Value, taskbar_home: bool, widget: &str) -> bool {
    (cfg!(windows) && is_on_desk(desk, widget)) || (taskbar_home && is_on_taskbar(taskbar, widget))
}

// Its runs: the parts each once, and up to 32 widgets, each once.
fn is_taskbar_layout(v: &Value) -> bool {
    let Some(runs) = v.as_object() else { return false };
    let mut seen: Vec<&str> = Vec::new();
    let mut widgets = 0;
    runs.iter().all(|(run, parts)| {
        matches!(run.as_str(), "left" | "center" | "right")
            && parts.as_array().is_some_and(|parts| {
                parts.iter().all(|p| match (p.as_str(), is_taskbar_widget(p)) {
                    (Some(p), _) if TASKBAR_PARTS.contains(&p) && !seen.contains(&p) => {
                        seen.push(p);
                        true
                    }
                    (None, Some(id)) if !seen.contains(&id) && widgets < 32 => {
                        seen.push(id);
                        widgets += 1;
                        true
                    }
                    _ => false,
                })
            })
    })
}

// What a settings patch may hold, checked: anything else is dropped.
fn is_ok(sh: &Shared, key: &str, v: &Value) -> bool {
    let is_bool = v.is_boolean();
    match key {
        "lang" => matches!(v.as_str(), Some("auto" | "zh" | "en")),
        "pet" => v.as_str().is_some_and(|id| data::pets(&sh.dir).iter().any(|p| p.id == id)),
        "scale" => v.as_f64().is_some_and(|s| data::SCALES.iter().any(|(_, x)| (x - s).abs() < 1e-9)),
        "details" | "walk" | "look" | "sound" | "dnd" | "hideInFullscreen" | "onboarded" | "out" | "settingsPin" | "settingsTop" | "oledShift" | "autoUpdateCheck" => is_bool,
        // Where an update is downloaded from first (updates.rs gitee_first).
        "updateFrom" => matches!(v.as_str(), Some("auto" | "github" | "gitee")),
        // Crash reports sent by themselves, never, or asked about (crash.rs).
        "crashReports" => matches!(v.as_str(), Some("auto" | "off" | "ask")),
        // The taskbar home stands in for Windows' own: elsewhere, only the island.
        "display" => v.as_str() == Some("island") || (cfg!(windows) && v.as_str() == Some("taskbar")),
        "islandWidth" => matches!(v.as_str(), Some("narrow" | "normal" | "wide")),
        // At the top of the screen, up out of the way of a window under it (island.rs look_under).
        "islandHideUnder" => is_bool,
        "taskbarMaterial" => matches!(v.as_str(), Some("mica" | "black" | "clear")),
        "taskbarClearWhen" => matches!(v.as_str(), Some("mica" | "solid" | "always")),
        "taskbarButtons" => matches!(v.as_str(), Some("icons" | "labels")),
        "taskbarAlign" => matches!(v.as_str(), Some("center" | "left")),
        "taskbarEdge" => matches!(v.as_str(), Some("bottom" | "top" | "left" | "right")),
        "taskbarDisplays" => matches!(v.as_str(), Some("main" | "all")),
        // The taskbar's parts in three runs (island.js tbLayout), each once.
        "taskbarLayout" => v.is_null() || is_taskbar_layout(v),
        // The desktop's widgets (desk.rs, deskgrid.js).
        "deskLayout" => is_desk_layout(v),
        // Boards kept as combos, to be put up again from the picker's 组合.
        "boardSets" => is_board_sets(v),
        // Her capsule's members, their order and how each shows there.
        "capsule" => is_capsule(v),
        // The programs' buttons in the order dragged: their keys (paths).
        "taskbarOrder" => v.as_array().is_some_and(|keys| keys.len() <= 200 && keys.iter().all(|k| k.as_str().is_some_and(|s| s.len() <= 1024))),
        "hold" => v.as_str() == Some("seen") || matches!(v.as_u64(), Some(8 | 30 | 120)),
        // When she has something to tell that needs no answer (a turn's end,
        // a hello, a plugin's nudge): the island open by itself, a ring round
        // it (the flow light), or only the dot (island.js cue).
        "endCue" => matches!(v.as_str(), Some("open" | "ring" | "none")),
        // Minutes a session at rest stays in sight, or 0: always (state.rs is_shown).
        "idleHideMin" => matches!(v.as_u64(), Some(0 | 15 | 30 | 60 | 120)),
        "promptWaitSec" => matches!(v.as_u64(), Some(30 | 60 | 120 | 290)),
        "notify" => ["waiting", "done", "error"].iter().all(|k| v.get(k).is_some_and(Value::is_boolean)),
        "widgetsOff" | "widgetOrder" => v.as_array().is_some_and(|ids| ids.len() <= 64 && ids.iter().all(|id| id.as_str().is_some_and(crate::widgets::is_id))),
        "widgetSpin" => matches!(v.as_u64(), Some(0 | 5 | 8 | 15)),
        // Each widget's row in the open island, small or medium, by its id
        // (island.js rowSize).
        "widgetSize" => v.as_object().is_some_and(|m| m.len() <= 64 && m.iter().all(|(id, s)| crate::widgets::is_id(id) && matches!(s.as_str(), Some("S" | "M")))),
        "plugins" => scripts::is_ok(v),
        "monitor" => v.as_object().is_some_and(|m| {
            m.iter().all(|(k, v)| match k.as_str() {
                "cpu" | "mem" | "net" | "battery" | "pin" => v.is_boolean(),
                "every" => v.as_u64().is_some_and(|s| (1..=10).contains(&s)),
                // The hardware's sensors: the built-in ones picked, and
                // LibreHardwareMonitor's.
                _ => crate::hw::is_setting(k, v).or_else(|| crate::lhm::is_setting(k, v)).unwrap_or(false),
            })
        }),
        // Where the monitor's sensors come from (hw.rs Source).
        "monitorSource" => crate::hw::is_source(v),
        "widgetNudge" => is_bool,
        // Not to be reminded that nothing is connected (island.js isHinting).
        "connectHintOff" => is_bool,
        // Her island's look on the taskbar, by Windows' mode (island.js
        // drawIslandLook): none, or a mode's none, is Windows' own grey.
        // And at the top of the screen (island.js lookNow), kept apart:
        // none is black, as it always was.
        "islandLook" | "islandTopLook" => v.is_null() || v.as_object().is_some_and(|m| m.iter().all(|(mode, look)| matches!(mode.as_str(), "dark" | "light") && is_island_look(look))),
        "mailAgent" => matches!(v.as_str(), Some("claude" | "codex" | "agy")),
        // The Mail page in two panes (list, letter), or one.
        "mailPanes" => matches!(v.as_str(), Some("one" | "two")),
        // How much mail is kept whole offline (mail/store.rs Offline).
        "mailOffline" => matches!(v.as_str(), Some("90d" | "year" | "all" | "opened")),
        "mailAgentConf" => mail::agent::is_conf_ok(v),
        // How mail reaches its servers (mail/proxy.rs).
        "mailProxy" => mail::proxy::Via::of(v).is_some(),
        _ => false,
    }
}

// A patch from the settings: checked, with the size, the display and her
// being out through the windows, so she keeps her spot.
pub fn apply_patch(sh: &Arc<Shared>, patch: &Value) -> Value {
    let mut rest = Map::new();
    let (mut scale, mut display, mut out) = (None, None, None);
    for (key, value) in patch.as_object().into_iter().flatten() {
        if !is_ok(sh, key, value) {
            continue;
        }
        match key.as_str() {
            "scale" => scale = value.as_f64(),
            "display" => display = value.as_str().map(str::to_string),
            "out" => out = value.as_bool(),
            // LibreHardwareMonitor's password, typed: to the Credential Manager.
            "monitor" => {
                rest.insert(key.clone(), crate::lhm::keep_password(value.clone()));
            }
            _ => {
                rest.insert(key.clone(), value.clone());
            }
        }
    }
    if let Some(scale) = scale {
        pet::resize(sh, scale);
    }
    if let Some(display) = display {
        set_display(sh, &display);
    }
    if let Some(out) = out {
        set_out(sh, out);
    }
    if rest.contains_key("dnd") && rest["dnd"] == true && sh.asks.lock().unwrap().dismiss_all() > 0 {
        sh.push_asks();
    }
    let plugins = rest.contains_key("plugins");
    let offline = rest.contains_key("mailOffline");
    let mail_way = rest.contains_key("mailProxy");
    let crash_auto = rest.get("crashReports").and_then(Value::as_str) == Some("auto");
    if !rest.is_empty() {
        sh.change(Value::Object(rest));
    }
    // Crash reports turned on: those waiting sent now.
    if crash_auto {
        crate::crash::send_now(sh);
    }
    if plugins && scripts::sync(sh) {
        sh.redraw();
    }
    // Mail kept offline for more or less: each account's sync at it now,
    // once the setting is there to be read.
    if offline {
        mail::kick_all(sh);
    }
    // Mail through another way: the accounts that follow the page's, connected again.
    if mail_way && mail::sync(sh) {
        sh.redraw();
    }
    snapshot(sh)
}

#[tauri::command]
pub async fn settings_get(app: AppHandle) -> Value {
    snapshot(&shared(&app))
}

#[tauri::command]
pub async fn settings_set(app: AppHandle, patch: Value) -> Value {
    apply_patch(&shared(&app), &patch)
}

// Open them from anywhere (the tray, her double-click): on a page, or where they were.
#[tauri::command]
pub async fn settings_open(app: AppHandle, tab: Option<String>) {
    island::open_settings(&shared(&app), tab.as_deref());
}

// The page closed them: the keyboard goes back, and a risen island goes.
#[tauri::command]
pub async fn settings_closed(app: AppHandle, window: WebviewWindow) {
    let sh = shared(&app);
    island::settings_closed(&sh);
    sh.set_keyboard(&window, false);
}

#[tauri::command]
pub async fn settings_show_pet(app: AppHandle, on: bool) -> Value {
    let sh = shared(&app);
    sh.set_hidden(!on);
    snapshot(&sh)
}

// Windows' own light or dark mode, or its accent colour ("#rrggbb"), set
// as Settings → Personalization → Colors sets them (theme.rs).
#[tauri::command]
pub async fn settings_win_look(app: AppHandle, light: Option<bool>, accent: Option<String>) -> Value {
    let sh = shared(&app);
    if let Some(light) = light {
        let ok = crate::theme::set_mode(light);
        sh.log(&format!("settings: Windows' mode set {}: {ok}", if light { "light" } else { "dark" }));
    }
    if let Some(rgb) = accent.as_deref().and_then(crate::theme::parse) {
        let picked = accent.unwrap_or_default();
        let (later_sh, later_picked) = (sh.clone(), picked.clone());
        let came = crate::theme::set_accent(rgb, move |note| later_sh.log(&format!("settings: Windows' accent {later_picked}: {note}")));
        sh.log(&format!("settings: Windows' accent set {picked}: {came}"));
    }
    snapshot(&sh)
}

// The desktop's picture (paper.rs): a picture or a folder picked in Windows'
// file dialog ("picture", "folder"), how it is laid ("position"), the
// folder's turns ("every", "shuffle"), the next or the one before ("next",
// "previous"); a change of these undone ("undo").
#[tauri::command]
pub async fn settings_wallpaper(app: AppHandle, what: String, value: Option<String>) -> Value {
    let sh = shared(&app);
    let owner = crate::island::hwnd(&sh);
    let lang = sh.lang();
    let t = move |key: &str| crate::i18n::t(lang, key);
    let what_done = what.clone();
    // What changes the picture is kept to be undone ("undo"), once picked.
    let changed = |done: bool| {
        crate::paper::remember();
        done
    };
    let done = tauri::async_runtime::spawn_blocking(move || match what.as_str() {
        "picture" => crate::paper::pick(false, &t("wallpaper.pickPicture"), &t("wallpaper.pictures"), owner).map(|p| (changed(true) && crate::paper::set_picture(&p), p)),
        "folder" => crate::paper::pick(true, &t("wallpaper.pickFolder"), "", owner).map(|p| (changed(true) && crate::paper::set_folder(&p), p)),
        "position" => value.map(|v| (changed(true) && crate::paper::set_position(&v), v)),
        "every" => value.map(|v| (crate::paper::set_turns(v.parse().ok(), None), v)),
        "shuffle" => value.map(|v| (crate::paper::set_turns(None, Some(v == "true")), v)),
        "next" => Some((changed(true) && crate::paper::next(), String::new())),
        "previous" => Some((changed(true) && crate::paper::previous(), String::new())),
        "undo" => Some((crate::paper::undo(), String::new())),
        _ => None,
    })
    .await
    .ok()
    .flatten();
    for line in crate::paper::notes() {
        sh.log(&format!("settings: wallpaper: {line}"));
    }
    if let Some((ok, value)) = done {
        sh.log(&format!("settings: wallpaper {what_done} {value}: {ok}"));
        // The taskbar's Mica follows the new picture: Windows says nothing
        // of a change made through IDesktopWallpaper, and the taskbar would
        // look on its own only every half minute. A moment after, for the
        // picture to be in place.
        if ok {
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(600));
                crate::taskbar::look_now();
            });
        }
    }
    snapshot(&sh)
}

#[tauri::command]
pub async fn settings_login(app: AppHandle, on: bool) -> Value {
    let sh = shared(&app);
    if let Err(err) = connection::set_open_at_login(on) {
        sh.log(&format!("start at login: {err}"));
    }
    snapshot(&sh)
}

// install, install-http, remove: Claude Code's settings.json.
#[tauri::command]
pub async fn settings_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_hooks(&action, sh.port) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: Codex's hooks.json.
#[tauri::command]
pub async fn settings_codex_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_codex_hooks(&action, &i18n::t(sh.lang(), "codex.waiting")) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: Antigravity's hooks.json.
#[tauri::command]
pub async fn settings_agy_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_agy_hooks(&action) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        // The folder she is in, for one whose path agy can't run (kind "path").
        Err(error) => {
            let folder = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|p| p.to_string_lossy().into_owned())).unwrap_or_default();
            json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "folder": folder, "snapshot": snapshot(&sh) })
        }
    }
}

// install, remove: our plugin in OpenCode's plugin folder.
#[tauri::command]
pub async fn settings_opencode(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_opencode_plugin(&action, sh.port) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: ZCode's config.json.
#[tauri::command]
pub async fn settings_zcode_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_zcode_hooks(&action) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: Trae CN's hooks.json.
#[tauri::command]
pub async fn settings_trae_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_trae_hooks(&action) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: WorkBuddy's settings.json.
#[tauri::command]
pub async fn settings_workbuddy_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_workbuddy_hooks(&action) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: omp's extension, in each of its agent folders.
#[tauri::command]
pub async fn settings_omp_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_omp_hooks(&action, sh.port) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// install, remove: DeepSeek Harness's hook bridge (its home-level patch file).
#[tauri::command]
pub async fn settings_dsh_hooks(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    match connection::write_dsh_hooks(&action) {
        Ok(()) => json!({ "ok": true, "snapshot": snapshot(&sh) }),
        Err(error) => json!({ "ok": false, "kind": connection::error_kind(&error), "error": error, "snapshot": snapshot(&sh) }),
    }
}

// The CPU temperature/power helper (sensors.rs, #18 part B). She is not
// elevated, so each action launches her own exe elevated (one UAC) and
// returns at once; the switch's state follows as the helper starts (or does
// not) answering. "install": set up the task (PawnIO must be present);
// "installPawnIO": download and install PawnIO first; "remove": take the
// task away (one more UAC) and leave PawnIO installed.
#[tauri::command]
pub async fn settings_cpu_sensors(app: AppHandle, action: String) -> Value {
    let sh = shared(&app);
    let exe = data::exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let sid = crate::logon::user_sid().unwrap_or_default();
    let data_dir = sh.dir.to_string_lossy().into_owned();
    let started = if action == "remove" {
        let params = format!("{} {} \"{}\"", crate::sensors::REMOVE_FLAG, sid, data_dir);
        crate::tasks::launch_as_admin_with(&exe, &params) > 32
    } else {
        let with_pawnio = if action == "installPawnIO" { " pawnio" } else { "" };
        let params = format!("{} {} {} \"{}\"{}", crate::sensors::SETUP_FLAG, sh.port, sid, data_dir, with_pawnio);
        let ok = crate::tasks::launch_as_admin_with(&exe, &params) > 32;
        if ok {
            crate::sensors::note_setting_up();
        }
        ok
    };
    // The person turned the UAC prompt down (ShellExecute gave at most 32).
    if started {
        json!({ "ok": true, "snapshot": snapshot(&sh) })
    } else {
        json!({ "ok": false, "kind": "cancelled", "snapshot": snapshot(&sh) })
    }
}

// Download a pet from codex-pets.net: the first one, or one replacing a
// missing sprite, becomes her.
#[tauri::command]
pub async fn settings_fetch(app: AppHandle, reference: String) -> Value {
    let sh = shared(&app);
    let dir = sh.dir.join("pets");
    let got = tauri::async_runtime::spawn_blocking(move || fetch::download_pet(&reference, &dir)).await;
    let lang = sh.lang();
    match got {
        Ok(Ok(pet)) => {
            let current = sh.setting("pet");
            let found = data::pets(&sh.dir);
            let has_current = found.iter().any(|p| Some(p.id.as_str()) == current.as_str());
            if !has_current || found.len() == 1 {
                sh.change(json!({ "pet": pet["id"] }));
            } else {
                sh.redraw();
            }
            let warning = pet["warning"]
                .as_object()
                .map(|w| fetch::describe(lang, &fetch::PetError { code: if w["code"] == "atlas" { "atlas" } else { "unknown" }, vars: w["vars"].clone() }));
            json!({ "ok": true, "pet": pet, "warning": warning, "snapshot": snapshot(&sh) })
        }
        Ok(Err(e)) => json!({ "ok": false, "error": fetch::describe(lang, &e), "snapshot": snapshot(&sh) }),
        Err(e) => json!({ "ok": false, "error": e.to_string(), "snapshot": snapshot(&sh) }),
    }
}

#[tauri::command]
pub async fn settings_gallery(app: AppHandle, page: Option<u64>, sort: Option<String>, query: Option<String>) -> Value {
    let sh = shared(&app);
    let sort = sort.unwrap_or_default();
    let query = query.unwrap_or_default();
    let got = tauri::async_runtime::spawn_blocking(move || fetch::gallery(page.unwrap_or(1), &sort, &query)).await;
    match got {
        Ok(Ok(page)) => json!({ "ok": true, "items": page["items"], "page": page["page"], "totalPages": page["totalPages"], "total": page["total"] }),
        Ok(Err(e)) => json!({ "ok": false, "error": fetch::describe(sh.lang(), &e) }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

// Her updates, from the General page: look now, the latest release's page,
// install it (updates.rs). The snapshot as it is then (a look or an
// install goes on on its own thread, pushed as it goes).
#[tauri::command]
pub async fn settings_update(app: AppHandle, what: String) -> Value {
    let sh = shared(&app);
    match what.as_str() {
        "check" => {
            let sh = sh.clone();
            std::thread::spawn(move || crate::updates::check_now(&sh));
            // Its "looking" shown at once.
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        "page" => crate::updates::open_page(&sh),
        "update" => crate::updates::update_now(&sh),
        _ => {}
    }
    snapshot(&sh)
}

// LibreHardwareMonitor for the monitor, from its settings (lhm_setup.rs):
// get (download, set up, start), start (one found: its web server turned
// on, started), startup and noStartup (its own task, at logon), usePort
// (hers set to the one its config says), folder (opened), look, dismiss.
// The snapshot as it is then (a job goes on on its own thread, pushed as it
// goes).
#[tauri::command]
pub async fn settings_lhm(app: AppHandle, what: String) -> Value {
    let sh = shared(&app);
    crate::lhm_setup::act(&sh, &what);
    snapshot(&sh)
}

// Her crash reports, from the General page: the answer to her asking
// (yes: sent from now on; no: never), sent now, or the last one's issue
// opened (crash.rs). The snapshot as it is then.
#[tauri::command]
pub async fn settings_crash(app: AppHandle, what: String) -> Value {
    let sh = shared(&app);
    match what.as_str() {
        "yes" => crate::crash::answer(&sh, true),
        "no" => crate::crash::answer(&sh, false),
        "send" => crate::crash::send_now(&sh),
        "open" => {
            if let Some(url) = crate::crash::open_last(&sh) {
                crate::jump::open_url(&url);
            }
        }
        _ => {}
    }
    snapshot(&sh)
}

// Only these places, whatever the page asks: the project, codex-pets.net,
// where a mail provider makes app passwords (the Mail page's hint), or how
// Antigravity's command line agy is installed (the Mail page, with
// Antigravity here but not agy).
#[tauri::command]
pub async fn settings_open_site(app: AppHandle, place: String) {
    let url = match place.as_str() {
        "repo" => REPO,
        "agyCli" => "https://antigravity.google/docs/cli/install",
        "googleAppPasswords" => "https://myaccount.google.com/apppasswords",
        "appleAppPasswords" => "https://account.apple.com/account/manage",
        "yahooAppPasswords" => "https://login.yahoo.com/account/security",
        "lhm" => "https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases",
        // Where to get an agent not on this computer (the Agent page, agents.js site).
        other => match other.strip_prefix("agent:").and_then(|id| AGENT_SITES.iter().find(|(a, _)| *a == id)) {
            Some((_, url)) => *url,
            None => fetch::SITE,
        },
    };
    // By ShellExecute, not `cmd /c start`: run from the installed pet (no
    // console), cmd got handles that were none and never started ("the
    // handle is invalid").
    if !crate::jump::open_url(url) {
        shared(&app).log(&format!("open {url}: ShellExecute failed"));
    }
}

// --- The dev servers she found (examples/widgets/devserver.ps1) -------------
// What the plugin kept in her data folder (devservers.json), and whether each
// answers now: the Plugins page's 开发服务 section. Starting one runs the
// plugin's -Start with the command it was seen with, out of her own job so
// the server does not go with her.
pub fn dev_servers(sh: &Shared) -> Value {
    let entries = std::fs::read_to_string(sh.dir.join("devservers.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut servers: Vec<Value> = entries
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|e| {
            let port = e.get("port").and_then(Value::as_u64)?;
            if !(1024..=65535).contains(&port) {
                return None;
            }
            let port = port as u16;
            let up = std::net::TcpStream::connect_timeout(
                &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                std::time::Duration::from_millis(250),
            )
            .is_ok();
            Some(json!({
                "port": port,
                "name": e.get("name").and_then(Value::as_str).unwrap_or(""),
                "project": e.get("project").and_then(Value::as_str).unwrap_or(""),
                "cmd": e.get("cmd").and_then(Value::as_str).unwrap_or(""),
                "up": up,
            }))
        })
        .collect();
    servers.sort_by_key(|s| s["port"].as_u64());
    let plugins = sh.setting("plugins");
    let on = plugins.get("devserver").and_then(|p| p.get("on")).and_then(Value::as_bool).unwrap_or(false);
    json!({ "on": on, "servers": servers })
}

#[tauri::command]
pub async fn settings_devservers(app: AppHandle) -> Value {
    dev_servers(&shared(&app))
}

// The shell the plugin runs with: PowerShell 7 where it is installed, else
// the one Windows keeps.
fn powershell() -> std::path::PathBuf {
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        let p = std::path::PathBuf::from(pf).join("PowerShell").join("7").join("pwsh.exe");
        if p.is_file() {
            return p;
        }
    }
    let sys = std::env::var_os("SystemRoot").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into());
    sys.join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe")
}

// Starting one kept as stopped: the plugin's -Start with the command it was
// seen with, out of her job (the MCP server and the settings' button both).
pub fn start_devserver(sh: &Shared, port: u16) -> bool {
    let script = sh.dir.join("plugins").join("devserver.ps1");
    if !script.is_file() {
        return false;
    }
    let args = [
        "-NoProfile".to_string(),
        "-NonInteractive".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-File".to_string(),
        script.to_string_lossy().into_owned(),
        "-Start".to_string(),
        port.to_string(),
        "-Port".to_string(),
        sh.port.to_string(),
    ];
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let ok = crate::detach::spawn(&powershell(), &refs);
    sh.log(&format!("devserver: start {port}: {ok}"));
    ok
}

#[tauri::command]
pub async fn settings_devserver_start(app: AppHandle, port: u16) -> Value {
    let sh = shared(&app);
    json!({ "ok": start_devserver(&sh, port) })
}

// Stopping one: the process listening on that port, ended (the row's 停止).
pub fn stop_devserver(sh: &Shared, port: u16) -> bool {
    if port < 1024 {
        return false;
    }
    let line = format!(
        "$p = (Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1).OwningProcess; if ($p) {{ Stop-Process -Id $p -Force }}"
    );
    let ok = crate::detach::spawn(&powershell(), &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &line]);
    sh.log(&format!("devserver: stop {port}: {ok}"));
    ok
}

#[tauri::command]
pub async fn settings_devserver_stop(app: AppHandle, port: u16) -> Value {
    let sh = shared(&app);
    json!({ "ok": stop_devserver(&sh, port) })
}

// --- CalDAV (iCloud first): the account, its password in Credential Manager,
// and the events pulled from it. Reading only for now: nothing is written to
// the person's own calendar until the pull is seen to be right.
const CALDAV_SECRET: &str = "wakuwaku-calendar";
const GOOGLE_SECRET: &str = "wakuwaku-calendar-google";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CALDAV: &str = "https://apidata.googleusercontent.com/caldav/v2/";

// Form encoding without pulling in the url crate: every value here is ASCII
// (client ids, tokens, codes, a loopback URL), so the unreserved set is enough.
fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v))).collect::<Vec<_>>().join("&")
}

fn url_decode(s: &str) -> String {
    let hex = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    };
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| url_decode(k) == key)
        .map(|(_, v)| url_decode(v))
}

fn calendar_account_file(sh: &Shared) -> std::path::PathBuf {
    sh.dir.join("calendar-account.json")
}

fn calendar_accounts_file(sh: &Shared) -> std::path::PathBuf {
    sh.dir.join("calendar-accounts.json")
}

fn save_calendar_accounts(sh: &Shared, accounts: &[Value], active: &str) {
    let _ = std::fs::write(calendar_accounts_file(sh), format!("{}\n", json!({ "accounts": accounts, "active": active })));
}

// Every account ever saved, in order; the one before this list existed is
// moved into it once, its secrets copied to their per-account names.
fn calendar_accounts(sh: &Shared) -> Vec<Value> {
    if let Ok(text) = std::fs::read_to_string(calendar_accounts_file(sh)) {
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            if let Some(list) = v["accounts"].as_array() {
                return list.clone();
            }
        }
    }
    let legacy: Value = std::fs::read_to_string(calendar_account_file(sh))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    if legacy["url"].as_str().unwrap_or("").is_empty() && legacy["user"].as_str().unwrap_or("").is_empty() {
        return Vec::new();
    }
    let id = format!("acc-{}", crate::now_ms());
    let mut account = legacy;
    account["id"] = json!(id);
    account["on"] = json!(true);
    let list = vec![account];
    save_calendar_accounts(sh, &list, &id);
    if let Some(secret) = crate::mail::secret::read_named(CALDAV_SECRET) {
        let _ = crate::mail::secret::write_named(&format!("{CALDAV_SECRET}-{id}"), &secret);
    }
    if let Some(secret) = crate::mail::secret::read_named(GOOGLE_SECRET) {
        let _ = crate::mail::secret::write_named(&format!("{GOOGLE_SECRET}-{id}"), &secret);
    }
    list
}

fn calendar_active(sh: &Shared, accounts: &[Value]) -> String {
    let wanted = std::fs::read_to_string(calendar_accounts_file(sh))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v["active"].as_str().map(String::from))
        .unwrap_or_default();
    accounts
        .iter()
        .find(|a| a["id"].as_str() == Some(wanted.as_str()) && a["on"].as_bool() != Some(false))
        .or_else(|| accounts.iter().find(|a| a["on"].as_bool() != Some(false)))
        .or_else(|| accounts.first())
        .and_then(|a| a["id"].as_str().map(String::from))
        .unwrap_or_default()
}

fn account_secret(id: &str, base: &str) -> Option<String> {
    crate::mail::secret::read_named(&format!("{base}-{id}")).or_else(|| crate::mail::secret::read_named(base))
}

// The address an account is reached at, out of whatever its form asked for:
// Nextcloud gives a server, a folder name and a calendar, and keeps the
// person from typing the path; Google gives a calendar id and nothing else.
// One place, so 测试连接 and 同步 always dial the same address (they did not:
// only the test button built Nextcloud's, the sync read the raw `url`, which
// Nextcloud's form never asks for, so its account could not be saved at all).
// The raw `url` still wins where it is one: a person who knows the address.
fn cal_account_url(account: &Value) -> String {
    let provider = account["provider"].as_str().unwrap_or("");
    let raw = account["url"].as_str().unwrap_or("").trim();
    let clean = |s: &str| s.trim().trim_end_matches('/').to_string();
    match provider {
        // Google's field asks for a calendar id, not a URL, so nothing here
        // adds a scheme to it. A real address in it means the person knows
        // where the server is (a proxy, say).
        "google" => {
            if raw.contains("://") {
                return clean(raw);
            }
            let id = if raw.is_empty() { account["user"].as_str().unwrap_or("").trim() } else { raw };
            if id.is_empty() {
                return String::new();
            }
            format!("{GOOGLE_CALDAV}{}/events", id.replace('#', "%23").replace(' ', "%20"))
        }
        "nextcloud" => {
            if !raw.is_empty() {
                return cal_server(raw);
            }
            let host = cal_server(account["host"].as_str().unwrap_or(""));
            if host.is_empty() {
                return String::new();
            }
            let mut out = host;
            let user = account["user"].as_str().unwrap_or("").trim();
            if !user.is_empty() {
                out.push_str(&format!("/remote.php/dav/calendars/{user}"));
            }
            let name = account["calendar"].as_str().unwrap_or("").trim();
            if !name.is_empty() {
                out.push_str(&format!("/{name}"));
            }
            out
        }
        _ => cal_server(raw),
    }
}

// A server address as typed. "cloud.example.com" is what people write, so a
// missing scheme becomes https://; never http, because the password goes in
// the Basic header and an account that asked for http:// is refused by the
// save with a line saying so.
fn cal_server(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    if s.is_empty() {
        return String::new();
    }
    if s.contains("://") {
        return s.to_string();
    }
    format!("https://{s}")
}

// The name signed in with, which is not always the one the path carries:
// Nextcloud's login name and the calendar folder under it are two fields in
// its form, and they are often different.
fn cal_login(account: &Value) -> String {
    let alt = account["account"].as_str().unwrap_or("").trim();
    if !alt.is_empty() {
        return alt.to_string();
    }
    account["user"].as_str().unwrap_or("").trim().to_string()
}

// The account's own proxy, or none: one address as typed ("127.0.0.1:7890"),
// a scheme before it saying which kind, an absent one HTTP as everywhere else
// in her. It was saved and nothing ever read it, so filling it in changed
// nothing at all.
fn cal_proxy_typed(account: &Value) -> Result<Option<Proxy>, String> {
    let text = account["proxy"].as_str().unwrap_or("").trim();
    if text.is_empty() {
        return Ok(None);
    }
    Proxy::parse(text, Kind::Http).map(Some).ok_or_else(|| format!("代理地址看不懂：{text}（形如 127.0.0.1:7890 或 socks5://127.0.0.1:10808）"))
}

// The proxy the account's calls go through: the one it names, or Windows'
// own, as the Mail page follows it by default: PowerShell 7 does not read
// the system proxy (only the environment's), and iCloud is out of reach from
// here without one.
fn cal_proxy(account: &Value) -> Result<Option<Proxy>, String> {
    match cal_proxy_typed(account)? {
        Some(p) => Ok(Some(p)),
        None => Ok(crate::proxy::system().and_then(|s| s.for_web()).and_then(|u| Proxy::parse(&u, Kind::Http))),
    }
}

// A proxy as a URL, which is what both sides of the calendar take:
// PowerShell's -Proxy, and ureq for Google's token.
fn proxy_url(p: &Proxy) -> String {
    match p.kind {
        Kind::Http => format!("http://{}", p.at()),
        Kind::Socks5 => format!("socks5://{}", p.at()),
    }
}

#[tauri::command]
pub async fn calendar_account(app: AppHandle) -> Value {
    let sh = shared(&app);
    let accounts = calendar_accounts(&sh);
    let active = calendar_active(&sh, &accounts);
    json!({ "accounts": accounts, "active": active })
}

#[tauri::command]
pub async fn calendar_account_save(app: AppHandle, account: Value, password: String) -> Value {
    let sh = shared(&app);
    let provider = account["provider"].as_str().unwrap_or("custom").to_string();
    let google = provider == "google";
    let url = cal_account_url(&account);
    if google {
        if url.is_empty() {
            return json!({ "ok": false, "error": "google: calendar id or email" });
        }
    } else if !url.starts_with("https://") || cal_login(&account).is_empty() {
        return json!({ "ok": false, "error": "地址要以 https:// 开头，账号也要填" });
    }
    let mut accounts = calendar_accounts(&sh);
    let id = account["id"].as_str().filter(|i| !i.is_empty()).map(String::from).unwrap_or_else(|| format!("acc-{}", crate::now_ms()));
    let mut saved = json!({ "id": id, "provider": provider, "url": account["url"].as_str().unwrap_or("").trim(), "user": account["user"].as_str().unwrap_or("").trim(), "on": account["on"].as_bool().unwrap_or(true) });
    if let Some(client) = account["clientId"].as_str().filter(|c| !c.is_empty()) {
        saved["clientId"] = json!(client);
    }
    // Not secrets: they may be read back into the form.
    for key in ["host", "calendar", "account", "tokenUrl", "proxy"] {
        if let Some(value) = account[key].as_str().map(str::trim).filter(|v| !v.is_empty()) {
            saved[key] = json!(value);
        }
    }
    // Secrets: the password, or Google's client secret and refresh token, go
    // to Credential Manager like the mail ones do and are never written into
    // the accounts file. Google's are kept together in one record, so the
    // OAuth trip and the hand-filled fields both land in the same place.
    if google {
        let mut held: Value = account_secret(&id, GOOGLE_SECRET)
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(Value::Null);
        // An account saved while the token used to be written into this file
        // keeps its token: it is taken from there once, into Credential
        // Manager, rather than dropped by the save that empties the field.
        let kept_token = |a: &Value| a["refreshToken"].as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
        let typed_token = account["refreshToken"].as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
        let stored_token = accounts.iter().find(|a| a["id"].as_str() == Some(id.as_str())).and_then(kept_token);
        if !password.is_empty() {
            held["clientSecret"] = json!(password);
        }
        if let Some(token) = typed_token.or(stored_token).or_else(|| kept_token(&held)) {
            held["refreshToken"] = json!(token);
        }
        if !held.is_null() {
            if let Err(e) = crate::mail::secret::write_named(&format!("{GOOGLE_SECRET}-{id}"), &held.to_string()) {
                return json!({ "ok": false, "error": e });
            }
        }
    } else if !password.is_empty() {
        if let Err(e) = crate::mail::secret::write_named(&format!("{CALDAV_SECRET}-{id}"), &password) {
            return json!({ "ok": false, "error": e });
        }
    }
    if let Some(index) = accounts.iter().position(|a| a["id"].as_str() == Some(id.as_str())) {
        accounts[index] = saved;
    } else {
        accounts.push(saved);
    }
    save_calendar_accounts(&sh, &accounts, &id);
    json!({ "ok": true, "id": id })
}

#[tauri::command]
pub async fn calendar_account_select(app: AppHandle, id: String) -> Value {
    let sh = shared(&app);
    let accounts = calendar_accounts(&sh);
    if !accounts.iter().any(|a| a["id"].as_str() == Some(id.as_str())) {
        return json!({ "ok": false, "error": "no account" });
    }
    save_calendar_accounts(&sh, &accounts, &id);
    json!({ "ok": true })
}

#[tauri::command]
pub async fn calendar_account_remove(app: AppHandle, id: String) -> Value {
    let sh = shared(&app);
    let mut accounts = calendar_accounts(&sh);
    accounts.retain(|a| a["id"].as_str() != Some(id.as_str()));
    let active = calendar_active(&sh, &accounts);
    save_calendar_accounts(&sh, &accounts, &active);
    json!({ "ok": true, "active": active })
}

// --- Google: OAuth once in the browser, then the same CalDAV client with a
// Bearer token that is refreshed at every sync. The client id lives with the
// account; the client secret and refresh token in Credential Manager.
// An access token for Google's CalDAV, from the refresh token taken in the
// OAuth browser trip or typed by hand: what was typed wins over what is
// kept, so 测试连接 can try an account before it is saved. The token URL is
// Google's unless the account names another (a proxy, say) — the field was
// saved and never read, so an account behind one could not sync at all.
fn google_token_with(account: &Value, typed_secret: &str, typed_refresh: &str, proxy: Option<&Proxy>) -> Result<String, String> {
    let client_id = account["clientId"].as_str().filter(|c| !c.is_empty()).ok_or("google: no client id")?;
    let held: Value = account_secret(account["id"].as_str().unwrap_or(""), GOOGLE_SECRET)
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    let secret = if !typed_secret.is_empty() {
        typed_secret.to_string()
    } else {
        held["clientSecret"].as_str().filter(|s| !s.is_empty()).ok_or("google: no client secret")?.to_string()
    };
    let refresh = if !typed_refresh.is_empty() {
        typed_refresh.to_string()
    } else {
        held["refreshToken"].as_str().filter(|r| !r.is_empty()).ok_or("google: not authorized")?.to_string()
    };
    let token_url = match account["tokenUrl"].as_str().map(str::trim).filter(|u| !u.is_empty()) {
        Some(u) => u.to_string(),
        None => GOOGLE_TOKEN_URL.to_string(),
    };
    let body = form_body(&[("client_id", client_id), ("client_secret", &secret), ("refresh_token", &refresh), ("grant_type", "refresh_token")]);
    // The same way through a proxy as mail takes it: HTTP's CONNECT, or
    // SOCKS5 with the server's name, so the name is never looked up here.
    let agent = mail::proxy::agent(proxy, std::time::Duration::from_secs(30));
    let mut res = agent
        .post(&token_url)
        .header("content-type", "application/x-www-form-urlencoded")
        .send(&body)
        .map_err(|e| format!("google token: {e}"))?;
    let answer: Value = res.body_mut().read_json().map_err(|e| format!("google token: {e}"))?;
    answer["access_token"].as_str().map(String::from).ok_or_else(|| "google token: no access_token".into())
}

fn google_token(account: &Value) -> Result<String, String> {
    let proxy = cal_proxy(account).ok().flatten();
    google_token_with(account, "", "", proxy.as_ref())
}

// PKCE: the verifier never leaves the machine; Google gets only its SHA-256.
// RandomState is seeded by the OS, which is the entropy available here.
fn pkce_pair() -> (String, String) {
    use base64::Engine as _;
    use std::hash::{BuildHasher, Hasher};
    let mut seed = Vec::with_capacity(32);
    for i in 0u64..4 {
        let state = std::collections::hash_map::RandomState::new();
        let mut h = state.build_hasher();
        h.write_u64(i);
        seed.extend_from_slice(&h.finish().to_le_bytes());
    }
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(seed);
    use sha2::Digest;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn google_auth_blocking(id: String, client_id: String, client_secret: String) -> Result<(), String> {
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();
    if id.trim().is_empty() {
        return Err("google: save the account first".into());
    }
    if client_id.is_empty() || client_secret.is_empty() {
        return Err("google: client id and secret".into());
    }
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server.server_addr().to_ip().map(|a| a.port()).ok_or("google: no callback port")?;
    let redirect = format!("http://127.0.0.1:{port}/oauth");
    let (verifier, challenge) = pkce_pair();
    let url = format!(
        "{GOOGLE_AUTH_URL}?{}",
        form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect),
            ("response_type", "code"),
            ("scope", "https://www.googleapis.com/auth/calendar"),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ])
    );
    let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err("google: timed out".into());
        }
        let request = match server.recv_timeout(left) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            Err(e) => return Err(e.to_string()),
        };
        let raw = request.url().to_string();
        if !raw.starts_with("/oauth") {
            let _ = request.respond(tiny_http::Response::empty(404));
            continue;
        }
        let query = raw.split_once('?').map(|(_, q)| q).unwrap_or("");
        let code = query_value(query, "code");
        let error = query_value(query, "error");
        let html = "<!doctype html><meta charset=\"utf-8\"><title>Wakuwaku</title><body style=\"font:16px system-ui;padding:3rem\">已回到 Wakuwaku，可以关闭这个页面了。</body>";
        let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
        let _ = request.respond(tiny_http::Response::from_string(html).with_header(header));
        if let Some(error) = error {
            return Err(format!("google: {error}"));
        }
        let Some(code) = code else { return Err("google: no code".into()) };
        let body = form_body(&[("code", &code), ("client_id", &client_id), ("client_secret", &client_secret), ("redirect_uri", &redirect), ("grant_type", "authorization_code"), ("code_verifier", &verifier)]);
        let agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(30))).build().new_agent();
        let mut res = agent
            .post(GOOGLE_TOKEN_URL)
            .header("content-type", "application/x-www-form-urlencoded")
            .send(&body)
            .map_err(|e| format!("google token: {e}"))?;
        let answer: Value = res.body_mut().read_json().map_err(|e| format!("google token: {e}"))?;
        let refresh = answer["refresh_token"].as_str().filter(|r| !r.is_empty()).ok_or("google: no refresh token (revoke the app's access and try again)")?;
        let saved = json!({ "clientSecret": client_secret, "refreshToken": refresh });
        crate::mail::secret::write_named(&format!("{GOOGLE_SECRET}-{id}"), &saved.to_string()).map_err(|e| e.to_string())?;
        return Ok(());
    }
}

#[tauri::command]
pub async fn calendar_google_auth(id: String, client_id: String, client_secret: String) -> Value {
    match tauri::async_runtime::spawn_blocking(move || google_auth_blocking(id, client_id, client_secret)).await {
        Ok(Ok(())) => json!({ "ok": true }),
        Ok(Err(e)) => json!({ "ok": false, "error": e }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

// A connection test: authenticate and PROPFIND only; nothing is written.
#[tauri::command]
pub async fn calendar_test(app: AppHandle, account: Value, password: String) -> Value {
    use base64::Engine as _;
    let sh = shared(&app);
    let provider = account["provider"].as_str().unwrap_or("custom");
    let url = cal_account_url(&account);
    if !url.starts_with("https://") {
        return json!({ "ok": false, "error": "地址还没填" });
    }
    let proxy = match cal_proxy(&account) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let proxy_arg = proxy.as_ref().map(proxy_url).unwrap_or_default();
    // Google's is a Bearer from its refresh token, typed or kept; the rest
    // sign in with a name and a password, Nextcloud's two fields apart.
    let auth = if provider == "google" {
        let typed_token = account["refreshToken"].as_str().unwrap_or("").trim();
        match google_token_with(&account, password.trim(), typed_token, proxy.as_ref()) {
            Ok(token) => format!("Bearer {token}"),
            Err(e) => return json!({ "ok": false, "error": e }),
        }
    } else {
        let login = cal_login(&account);
        let secret = if password.is_empty() {
            account_secret(account["id"].as_str().unwrap_or(""), CALDAV_SECRET).unwrap_or_default()
        } else {
            password
        };
        if login.is_empty() {
            return json!({ "ok": false, "error": "账号还没填" });
        }
        if secret.is_empty() {
            return json!({ "ok": false, "error": "密码还没填" });
        }
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{login}:{secret}")))
    };
    let propfind = "<?xml version=\"1.0\" encoding=\"utf-8\"?><d:propfind xmlns:d=\"DAV:\"><d:prop><d:displayname/><d:resourcetype/></d:prop></d:propfind>";
    match caldav_call(&sh, "PROPFIND", url.trim_end_matches('/'), &auth, propfind, &proxy_arg) {
        Some(xml) => {
            let base = url.split('/').take(3).collect::<Vec<_>>().join("/");
            let calendars = caldav_calendar_hrefs(&xml, &base).len().max(1);
            json!({ "ok": true, "calendars": calendars })
        }
        None => json!({ "ok": false, "error": "连不上：检查地址、账号和密码" }),
    }
}

// A calendar event's link: only http(s), opened by the system browser.
#[tauri::command]
pub async fn settings_open_external(url: String) -> Value {
    let url = url.trim();
    let ok = (url.starts_with("https://") || url.starts_with("http://")) && !url.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'');
    if !ok {
        return json!({ "ok": false, "error": "http(s) url" });
    }
    #[cfg(windows)]
    let opened = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn().is_ok();
    #[cfg(not(windows))]
    let opened = std::process::Command::new("xdg-open").arg(url).spawn().is_ok();
    json!({ "ok": opened })
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#13;", "\r").replace("&amp;", "&")
}

// PowerShell's -Proxy, or nothing. PowerShell 7 runs on .NET 6, whose
// HttpClient speaks SOCKS5, so both kinds of proxy reach the server the way
// the account asks; an empty WK_PROXY leaves Invoke-WebRequest on its own.
const PS_PROXY: &str = r#"if ($env:WK_PROXY) { $a['Proxy']=$env:WK_PROXY }"#;

// What the server answered, as UTF-8 on the standard output, not as the
// console writes it: the console's own code page cannot hold a Chinese
// character and writes a question mark for it, so the title of an event came
// back as "????" and — worse — was written back to the server that way. The
// bytes go out as they are, and Rust reads them as UTF-8.
const PS_WRITE_UTF8: &str = r#"$b=[System.Text.Encoding]::UTF8.GetBytes($Content); [Console]::OpenStandardOutput().Write($b,0,$b.Length)"#;

fn caldav_call(sh: &Shared, method: &str, url: &str, auth: &str, body: &str, proxy: &str) -> Option<String> {
    // The answer, tried again once: the connection to the server drops now and
    // then (the TLS handshake ending early, the stream ending before the
    // whole answer), and one more try usually gets through. Only the reads
    // are tried again: a write that is tried again may be written twice.
    for attempt in 0..2 {
        match caldav_call_once(sh, method, url, auth, body, proxy) {
            Some(text) => return Some(text),
            None if attempt == 0 => {
                sh.log(&format!("calendar: {method} {url} did not answer; tried again"));
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
            None => return None,
        }
    }
    None
}

fn caldav_call_once(sh: &Shared, method: &str, url: &str, auth: &str, body: &str, proxy: &str) -> Option<String> {
    let script = format!(
        r#"$h=@{{Authorization=$env:WK_AUTH;Depth='1'}}; $a=@{{}}; {PS_PROXY}; $r=Invoke-WebRequest -CustomMethod $env:WK_METHOD -Uri $env:WK_URL -Headers $h -ContentType 'application/xml; charset=utf-8' -Body $env:WK_BODY -TimeoutSec 30 @a; $Content=$r.Content; {PS_WRITE_UTF8}"#
    );
    let mut command = std::process::Command::new(powershell());
    command
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .env("WK_AUTH", auth)
        .env("WK_METHOD", method)
        .env("WK_URL", url)
        .env("WK_BODY", body)
        .env("WK_PROXY", proxy)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.trim().is_empty() {
        // What PowerShell said about it, if anything: without it a failed
        // call was just "no answer", and there was nothing to tell what was
        // wrong with it.
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if !said.is_empty() {
            sh.log(&format!("calendar: {method} {url} said: {}", &said[..said.len().min(300)]));
        }
        return None;
    }
    Some(text)
}

// The responses of a multistatus, whatever the server calls the tag: DAV:
// is often the default namespace and the elements come as <response>, and
// often not and they come as <d:response>. Both are read the same, and
// before this only the first spelling was read, so a server that used the
// second was read as one long response and nothing came of it.
fn caldav_blocks(xml: &str) -> Vec<&str> {
    let mut starts: Vec<usize> = Vec::new();
    let mut at = 0usize;
    while at < xml.len() {
        let Some(found) = xml[at..].find("response") else { break };
        let name_at = at + found;
        at = name_at + 8;
        // The tag the word is in: "<response>", "<d:response>", "<D:response
        // >". What comes before it may be a prefix or nothing.
        let open = xml[..name_at].rfind('<').unwrap_or(0);
        let prefix = &xml[open + 1..name_at];
        if !prefix.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_')) {
            continue;
        }
        // And the word must be the whole name: "responsed" is another tag.
        if !matches!(xml.as_bytes().get(name_at + 8), Some(b'>') | Some(b' ') | Some(b'/') | Some(b'\t')) {
            continue;
        }
        starts.push(open);
    }
    let mut out = Vec::new();
    for (i, s) in starts.iter().enumerate() {
        let e = starts.get(i + 1).copied().unwrap_or(xml.len());
        out.push(&xml[*s..e]);
    }
    out
}

// Whether a response is a calendar and not a plain collection: the
// resourcetype holds a calendar element in it. The element may be prefixed
// (<c:calendar/>, Apple's <apple:calendar/>) or not (<calendar/>), and the
// server's own spelling of it is what is looked at.
fn has_calendar(block: &str) -> bool {
    if block.contains("<calendar") {
        return true;
    }
    let Some(at) = block.find("resourcetype") else { return false };
    let rest = &block[at..];
    let end = rest.find("</").unwrap_or(rest.len());
    rest[..end].contains("calendar")
}

fn caldav_calendar_hrefs(xml: &str, base: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in caldav_blocks(xml) {
        if !has_calendar(block) {
            continue;
        }
        if let Some(href) = xml_tag_text(block, "href") {
            let href = href.trim();
            if href.is_empty() {
                continue;
            }
            out.push(if href.starts_with("http") { href.to_string() } else { format!("{}{}", base.trim_end_matches('/'), href) });
        }
    }
    out
}

// One calendar of an account: where it is, what it is called, and the
// colour it gave itself (CalDAV's calendar-color, as iCloud, Google and
// Nextcloud each answer it; the colour's own name, so a server that does
// not answer one is not refused over it).
#[derive(Clone, Debug, Default, PartialEq)]
struct CalCalendar {
    href: String,
    name: String,
    color: String,
}

// The calendars an account holds, with their names and colours. The colour
// is asked for under both names it comes by: CalDAV's own and Apple's, and
// whichever answers is taken. A colour of eight hex digits (Nextcloud's
// RGBA) is read as the first six.
fn caldav_calendars(xml: &str, base: &str) -> Vec<CalCalendar> {
    let mut out = Vec::new();
    for block in caldav_blocks(xml) {
        if !has_calendar(block) {
            continue;
        }
        let Some(href) = xml_tag_text(block, "href") else { continue };
        let href = href.trim();
        if href.is_empty() {
            continue;
        }
        let href = if href.starts_with("http") { href.to_string() } else { format!("{}{}", base.trim_end_matches('/'), href) };
        let name = xml_tag_text(block, "displayname").map(|s| s.trim().to_string()).unwrap_or_default();
        let mut color = String::new();
        for tag in ["calendar-color", "calendarColor"] {
            if let Some(value) = xml_tag_text(block, tag) {
                let value = value.trim().trim_start_matches('#').to_string();
                if value.len() >= 6 && value.chars().take(6).all(|c| c.is_ascii_hexdigit()) {
                    color = value.chars().take(6).collect::<String>().to_ascii_uppercase();
                    break;
                }
            }
        }
        out.push(CalCalendar { href, name, color });
    }
    out
}

fn ps_run(script: &str, envs: &[(&str, &str)]) -> Option<String> {
    let mut command = std::process::Command::new(powershell());
    command
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    for (name, value) in envs {
        command.env(name, value);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command.output().ok().map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// A PUT or DELETE; answers `ok|<etag>` or `no|<why>`.
fn caldav_send(method: &str, url: &str, auth: &str, body: &str, ctype: &str, cond: Option<(&str, &str)>, proxy: &str) -> Option<String> {
    let script = format!(
        r#"$h=@{{Authorization=$env:WK_AUTH}}; if ($env:WK_COND_NAME) {{ $h[$env:WK_COND_NAME]=$env:WK_COND_VAL }}; $a=@{{}}; {PS_PROXY}; try {{ $r=Invoke-WebRequest -Method $env:WK_METHOD -Uri $env:WK_URL -Headers $h -ContentType $env:WK_TYPE -Body $env:WK_BODY -TimeoutSec 30 @a; $Content='ok|' + [string]($r.Headers['ETag'] -join ' ') }} catch {{ $Content='no|' + $_.Exception.Message }}; {PS_WRITE_UTF8}"#
    );
    let (cond_name, cond_val) = cond.unwrap_or(("", ""));
    ps_run(
        &script,
        &[("WK_AUTH", auth), ("WK_METHOD", method), ("WK_URL", url), ("WK_BODY", body), ("WK_TYPE", ctype), ("WK_COND_NAME", cond_name), ("WK_COND_VAL", cond_val), ("WK_PROXY", proxy)],
    )
}

fn caldav_delete(url: &str, auth: &str, proxy: &str) -> bool {
    let script = format!(r#"$h=@{{Authorization=$env:WK_AUTH}}; $a=@{{}}; {PS_PROXY}; try {{ Invoke-WebRequest -Method Delete -Uri $env:WK_URL -Headers $h -TimeoutSec 30 @a | Out-Null; [Console]::Out.Write('ok') }} catch {{ [Console]::Out.Write('no') }}"#);
    ps_run(&script, &[("WK_AUTH", auth), ("WK_URL", url), ("WK_PROXY", proxy)]).is_some_and(|text| text == "ok")
}

fn caldav_get(url: &str, auth: &str, proxy: &str) -> Option<String> {
    // Tried again once, like the other reads: the connection drops now and
    // then, and one more try usually gets through.
    for attempt in 0..2 {
        match caldav_get_once(url, auth, proxy) {
            Some(text) => return Some(text),
            None if attempt == 0 => std::thread::sleep(std::time::Duration::from_millis(700)),
            None => return None,
        }
    }
    None
}

fn caldav_get_once(url: &str, auth: &str, proxy: &str) -> Option<String> {
    let script = format!(r#"$h=@{{Authorization=$env:WK_AUTH}}; $a=@{{}}; {PS_PROXY}; try {{ $r=Invoke-WebRequest -Method Get -Uri $env:WK_URL -Headers $h -TimeoutSec 30 @a; $Content=$r.Content }} catch {{ $Content='' }}; {PS_WRITE_UTF8}"#);
    ps_run(&script, &[("WK_AUTH", auth), ("WK_URL", url), ("WK_PROXY", proxy)]).filter(|text| !text.trim().is_empty())
}

#[derive(Clone)]
struct CaldavItem {
    id: String,
    href: String,
    etag: String,
    event: Value,
}

// One resource per item: its href, its ETag (what a later PUT must match) and
// the event inside. A recurring event can hold more than one VEVENT; they share
// the href and the ETag.
fn xml_tag_text(block: &str, name: &str) -> Option<String> {
    let pos = block.find(name)?;
    let after = &block[pos + name.len()..];
    let gt = after.find('>')?;
    let rest = &after[gt + 1..];
    let end = rest.find('<').unwrap_or(rest.len());
    Some(xml_unescape(rest[..end].trim()))
}

fn caldav_items(xml: &str, base: &str) -> Vec<CaldavItem> {
    let mut out = Vec::new();
    for block in caldav_blocks(xml) {
        let Some(href) = xml_tag_text(block, "href") else { continue };
        let href = href.trim();
        if href.is_empty() {
            continue;
        }
        let origin = base.split('/').take(3).collect::<Vec<_>>().join("/");
        let href = if href.starts_with("http") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{origin}{href}")
        } else {
            format!("{}/{}", base.trim_end_matches('/'), href)
        };
        let etag = xml_tag_text(block, "getetag").unwrap_or_default();
        let Some(start) = block.find("calendar-data") else { continue };
        let after = &block[start..];
        let Some(open_end) = after.find('>') else { continue };
        let content = &after[open_end + 1..];
        let Some(close) = content.find("</") else { continue };
        for event in ics_events(&xml_unescape(&content[..close])) {
            let id = event["id"].as_str().unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            out.push(CaldavItem { id, href: href.clone(), etag: etag.clone(), event });
        }
    }
    out
}

// An event's own href on the server, as the last pull saw it.
fn item_href(remote: &std::collections::HashMap<String, CaldavItem>, id: &str) -> String {
    remote.get(id).map(|i| i.href.clone()).unwrap_or_default()
}

// Whether a text is broken: a character that stands for one that could not
// be read (U+FFFD), or a run of question marks where a title was. Both came
// of a response written out in the console's own code page, which cannot
// hold a Chinese character and wrote a question mark (or bytes that are not
// UTF-8 at all) for it.
fn text_broken(s: &str) -> bool {
    if s.contains('\u{FFFD}') {
        return true;
    }
    s.as_bytes().windows(3).any(|w| w == b"???")
}

// An event whose own text is broken: its title, its note or where it is.
// The title is what a person reads; a broken one means the event is not
// what it was.
fn event_broken(e: &Value) -> bool {
    ["title", "note", "where"].iter().any(|k| text_broken(e[*k].as_str().unwrap_or("")))
}

fn event_sig(e: &Value) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        e["date"].as_str().unwrap_or(""),
        e["start"].as_str().unwrap_or(""),
        e["end"].as_str().unwrap_or(""),
        e["title"].as_str().unwrap_or(""),
        e["note"].as_str().unwrap_or(""),
        e["where"].as_str().unwrap_or(""),
        e["url"].as_str().unwrap_or(""),
        e["repeat"].as_str().unwrap_or("none"),
        e["remind"].as_u64().unwrap_or(0),
        e["remindDays"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
    )
}

// An event's identity among all the accounts: the account it is in and its
// own id. The same event may be in two accounts (copied to both), so the id
// alone does not say which one; the page works the same way (calEventKey in
// settings.js).
fn cal_event_key(event: &Value) -> Option<String> {
    let id = event["id"].as_str().filter(|i| !i.is_empty())?;
    let account = event["account"].as_str().unwrap_or("");
    Some(format!("{account}\u{1}{id}"))
}

// --- Reminders: a line on the island when an event comes within its notice ---
// Checked every 20 seconds; what has been said is kept so it is said once.

pub fn start_reminders(sh: std::sync::Arc<crate::Shared>) {
    crate::spawn_named("calendar-reminders", move || loop {
        remind_due(&sh);
        std::thread::sleep(std::time::Duration::from_secs(20));
    });
}

fn parse_date(s: &str) -> Option<(i64, i64, i64)> {
    let mut parts = s.split('-');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn parse_time(s: &str) -> Option<(i64, i64)> {
    let (h, m) = s.split_once(':')?;
    let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    (h >= 0 && h < 24 && m >= 0 && m < 60).then_some((h, m))
}

// Days since 1970-01-01 (the weekday is (days + 4) mod 7, Sunday 0).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn occurs_on(e: &Value, ty: i64, tm: i64, td: i64) -> bool {
    let Some((y, m, d)) = e["date"].as_str().and_then(parse_date) else { return false };
    let from = days_from_civil(y, m, d);
    let to = days_from_civil(ty, tm, td);
    if to < from {
        return false;
    }
    match e["repeat"].as_str().unwrap_or("none") {
        "daily" => true,
        "weekly" => (to - from) % 7 == 0,
        "monthly" => d == td,
        _ => to == from,
    }
}

// Days since 1970 back to a date (Howard Hinnant's civil_from_days, as
// crash.rs and widgets.rs have it).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2), m, d)
}

fn date_string(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_in_month(y: i64, m: i64) -> i64 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)
}

// The next month with that day in it: a monthly event on the 31st has no
// occurrence in a 30-day month, nor in February.
fn next_month_day(day: i64, wanted: i64) -> Option<i64> {
    let (mut y, mut m, _) = civil_from_days(day);
    for _ in 0..240 {
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
        if wanted <= days_in_month(y, m) {
            return Some(days_from_civil(y, m, wanted));
        }
    }
    None
}

// The first day this event is on, on or after today, in days since 1970.
fn next_day(e: &Value, today: i64) -> Option<i64> {
    let (y, m, d) = e["date"].as_str().and_then(parse_date)?;
    let from = days_from_civil(y, m, d);
    match e["repeat"].as_str().unwrap_or("none") {
        "daily" => Some(today.max(from)),
        "weekly" => {
            if today <= from {
                Some(from)
            } else {
                Some(from + ((today - from + 6) / 7) * 7)
            }
        }
        "monthly" => {
            if from >= today {
                return Some(from);
            }
            let mut day = from;
            for _ in 0..240 {
                day = next_month_day(day, d)?;
                if day >= today {
                    return Some(day);
                }
            }
            None
        }
        _ => (from >= today).then_some(from),
    }
}

// An occurrence and the instant it begins: a timed event at its clock time,
// an all-day one at 09:00 (the hour a calendar on a phone uses for it).
struct Due {
    date: String,
    start: String,
    at: i64,
}

// The next occurrence that has not already begun, or, for the reminder set
// for the event's own time, one that began within its grace (an hour, or the
// rest of an all-day event's day) so a check every twenty seconds cannot
// step over it.
fn next_occurrence(e: &Value, today: i64, midnight: i64, now: i64, remind_days: i64) -> Option<Due> {
    let (_, _, d) = e["date"].as_str().and_then(parse_date)?;
    let repeat = e["repeat"].as_str().unwrap_or("none");
    let all_day = e["start"].as_str().unwrap_or("").trim().is_empty();
    let (h, min) = if all_day { (9, 0) } else { e["start"].as_str().and_then(parse_time).unwrap_or((9, 0)) };
    let start_min = h * 60 + min;
    let grace = if remind_days != 0 {
        0
    } else if all_day {
        86_400_000 - start_min * 60_000
    } else {
        3_600_000
    };
    let mut day = next_day(e, today)?;
    for _ in 0..400 {
        let at = midnight + (day - today) * 86_400_000 + start_min * 60_000;
        if at + grace > now {
            return Some(Due {
                date: date_string(day),
                start: if all_day { String::new() } else { format!("{h:02}:{min:02}") },
                at,
            });
        }
        day = match repeat {
            "daily" => day + 1,
            "weekly" => day + 7,
            "monthly" => next_month_day(day, d)?,
            _ => return None,
        };
    }
    None
}

fn remind_due(sh: &std::sync::Arc<crate::Shared>) {
    let Ok(text) = std::fs::read_to_string(sh.dir.join("calendar.json")) else { return };
    let Ok(Value::Array(events)) = serde_json::from_str::<Value>(&text) else { return };
    if events.is_empty() {
        return;
    }
    let today = crate::widgets::local_date();
    let Some((ty, tm, td)) = parse_date(&today) else { return };
    let today_days = days_from_civil(ty, tm, td);
    let midnight = crate::widgets::local_midnight() as i64;
    let now = crate::now_ms() as i64;
    let minutes = (now.saturating_sub(midnight) / 60_000) as i64;
    let path = sh.dir.join("calendar-notified.json");
    let mut sent: Vec<String> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .unwrap_or_default();
    let mut added = false;
    for e in &events {
        let id = e["id"].as_str().unwrap_or("");
        let title = e["title"].as_str().unwrap_or("");
        // The person's own setting, in days: 0 is the event's own time, n is
        // n days before it, and absent leaves the older minute setting.
        if let Some(days) = e["remindDays"].as_u64().map(|d| d.min(365) as i64) {
            let Some(due) = next_occurrence(e, today_days, midnight, now, days) else { continue };
            let remind_at = due.at - days * 86_400_000;
            if now < remind_at {
                continue;
            }
            let key = format!("{id}@{}@d{days}", due.date);
            if sent.contains(&key) {
                continue;
            }
            crate::toast::calendar(sh, &due.date, &due.start, &title);
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
            for b in key.bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100_0000_01b3);
            }
            let widget = json!({
                "id": format!("cal-{hash:x}"),
                "icon": "calendar",
                "color": "#5e9bff",
                "label": if sh.lang() == "en" { "Reminder" } else { "日程提醒" },
                "value": title,
                "detail": { "extra": if due.start.is_empty() { due.date.clone() } else { due.start.clone() } },
                "ttl": 1200,
                "nudge": true,
            });
            let _ = sh.put_widget(&widget);
            sh.log(&format!("calendar: reminded {key}"));
            sent.push(key);
            added = true;
            continue;
        }
        // The earlier setting: minutes before a timed event today.
        let remind = e["remind"].as_u64().unwrap_or(0).min(10_080) as i64;
        if remind == 0 {
            continue;
        }
        let Some((h, m)) = e["start"].as_str().and_then(parse_time) else { continue };
        if !occurs_on(e, ty, tm, td) {
            continue;
        }
        let start = h * 60 + m;
        if minutes < start - remind || minutes >= start {
            continue;
        }
        let key = format!("{id}@{today}");
        if sent.contains(&key) {
            continue;
        }
        crate::toast::calendar(sh, &today, &format!("{h:02}:{m:02}"), &title);
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for b in key.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        let widget = json!({
            "id": format!("cal-{hash:x}"),
            "icon": "calendar",
            "color": "#5e9bff",
            "label": if sh.lang() == "en" { "Reminder" } else { "日程提醒" },
            "value": title,
            "detail": { "extra": format!("{h:02}:{m:02}") },
            "ttl": 1200,
            "nudge": true,
        });
        let _ = sh.put_widget(&widget);
        sh.log(&format!("calendar: reminded {key}"));
        sent.push(key);
        added = true;
    }
    if added {
        if sent.len() > 200 {
            sent.drain(0..sent.len() - 200);
        }
        let _ = std::fs::write(&path, format!("{}\n", json!(sent)));
    }
}

// The island's calendar widget: the event today that is on now or the next one.
pub fn calendar_words(sh: &crate::Shared) -> Option<Value> {
    let text = std::fs::read_to_string(sh.dir.join("calendar.json")).ok()?;
    let events: Vec<Value> = serde_json::from_str(&text).ok()?;
    let today = crate::widgets::local_date();
    let (ty, tm, td) = parse_date(&today)?;
    let minutes = (crate::now_ms().saturating_sub(crate::widgets::local_midnight()) / 60_000) as i64;
    Some(json!(next_today_words(&events, ty, tm, td, minutes, sh.lang())?))
}

fn next_today_words(events: &[Value], ty: i64, tm: i64, td: i64, minutes: i64, lang: &str) -> Option<String> {
    let mut best: Option<(i64, String, String)> = None;
    for e in events {
        if !occurs_on(e, ty, tm, td) {
            continue;
        }
        let Some((h, m)) = e["start"].as_str().and_then(parse_time) else { continue };
        let start = h * 60 + m;
        let end = e["end"].as_str().and_then(parse_time).map(|(h, m)| h * 60 + m).unwrap_or(start).max(start);
        if end < minutes {
            continue;
        }
        let title: String = e["title"].as_str().unwrap_or("").chars().take(16).collect();
        let clock = e["start"].as_str().unwrap_or("").to_string();
        if best.as_ref().is_none_or(|(s, _, _)| start < *s) {
            best = Some((start, title, clock));
        }
    }
    let (start, title, clock) = best?;
    let now = if lang == "en" { "now" } else { "进行中" };
    let value = if start <= minutes { format!("{title} · {now}") } else { format!("{clock} {title}") };
    Some(value.chars().take(24).collect())
}

// --- For agents (Wakuwaku MCP): a day's agenda, and one added event --------

pub fn calendar_agenda(sh: &crate::Shared, date: &str) -> Value {
    let date = if date.trim().is_empty() { crate::widgets::local_date() } else { date.trim().to_string() };
    let Some((y, m, d)) = parse_date(&date) else {
        return json!({ "ok": false, "error": "date: YYYY-MM-DD" });
    };
    let events: Vec<Value> = std::fs::read_to_string(sh.dir.join("calendar.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<Value>>(&text).ok())
        .unwrap_or_default();
    let mut day: Vec<Value> = events.into_iter().filter(|e| occurs_on(e, y, m, d)).collect();
    day.sort_by(|a, b| a["start"].as_str().unwrap_or("").cmp(b["start"].as_str().unwrap_or("")));
    json!({ "ok": true, "date": date, "events": day })
}

pub fn calendar_add(sh: &crate::Shared, input: &Value) -> Result<Value, String> {
    let mut event = event_from_input(input)?;
    event["id"] = json!(crate::now_ms());
    let path = sh.dir.join("calendar.json");
    let mut events: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<Value>>(&text).ok())
        .unwrap_or_default();
    events.push(event.clone());
    std::fs::write(&path, format!("{}\n", json!(events))).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "event": event }))
}

fn event_from_input(input: &Value) -> Result<Value, String> {
    let title: String = input["title"].as_str().unwrap_or("").trim().chars().take(120).collect();
    if title.is_empty() {
        return Err("title is wanted".into());
    }
    let date = input["date"].as_str().map(str::trim).filter(|d| !d.is_empty()).map(String::from).unwrap_or_else(crate::widgets::local_date);
    if parse_date(&date).is_none() {
        return Err("date: YYYY-MM-DD".into());
    }
    let clock = |key: &str| -> Result<String, String> {
        let value = input[key].as_str().unwrap_or("").trim();
        if value.is_empty() {
            return Ok(String::new());
        }
        parse_time(value).map(|(h, m)| format!("{h:02}:{m:02}")).ok_or_else(|| format!("{key}: HH:MM"))
    };
    let start = clock("start")?;
    let end = clock("end")?;
    if !start.is_empty() && !end.is_empty() && end <= start {
        return Err("end must be after start".into());
    }
    let repeat = input["repeat"].as_str().unwrap_or("none");
    if !matches!(repeat, "none" | "daily" | "weekly" | "monthly") {
        return Err("repeat: none, daily, weekly or monthly".into());
    }
    let remind_days = input["remindDays"]
        .as_u64()
        .or_else(|| input["remindDays"].as_str().and_then(|s| s.trim().parse().ok()))
        .map(|d| d.min(365));
    let remind = if remind_days.is_some() { 0 } else { input["remind"].as_u64().unwrap_or(0).min(10_080) };
    let note: String = input["note"].as_str().unwrap_or("").trim().chars().take(200).collect();
    let place: String = input["where"].as_str().unwrap_or("").trim().chars().take(120).collect();
    let link: String = input["url"].as_str().unwrap_or("").trim().chars().take(300).collect();
    let mut event = json!({
        "date": date,
        "start": start,
        "end": end,
        "title": title,
        "note": note,
        "where": place,
        "url": link,
        "repeat": repeat,
        "remind": remind,
    });
    if let Some(days) = remind_days {
        event["remindDays"] = json!(days);
    }
    Ok(event)
}

fn same_id(e: &Value, id: &str) -> bool {
    e["id"].as_str() == Some(id) || e["id"].as_u64().is_some_and(|n| n.to_string() == id)
}

fn input_id(input: &Value) -> Result<String, String> {
    match &input["id"] {
        Value::String(s) if !s.is_empty() => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err("id is wanted".into()),
    }
}

fn apply_update(old: &Value, input: &Value) -> Result<Value, String> {
    let mut merged = old.clone();
    for key in ["title", "date", "start", "end", "note", "where", "url", "repeat", "remind", "remindDays"] {
        if let Some(given) = input.get(key) {
            merged[key] = given.clone();
        }
    }
    // One setting replaces the other: days when days are given, minutes when
    // minutes are. An update that names only one clears what the other left.
    if input.get("remindDays").is_some() && input.get("remind").is_none() {
        merged["remind"] = json!(0);
    }
    if input.get("remind").is_some() && input.get("remindDays").is_none() {
        if let Some(object) = merged.as_object_mut() {
            object.remove("remindDays");
        }
    }
    let mut event = event_from_input(&merged)?;
    event["id"] = old["id"].clone();
    Ok(event)
}

pub fn calendar_update(sh: &crate::Shared, input: &Value) -> Result<Value, String> {
    let id = input_id(input)?;
    let path = sh.dir.join("calendar.json");
    let mut events: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<Value>>(&text).ok())
        .unwrap_or_default();
    let Some(index) = events.iter().position(|e| same_id(e, &id)) else {
        return Err("no event with that id".into());
    };
    let event = apply_update(&events[index], input)?;
    events[index] = event.clone();
    std::fs::write(&path, format!("{}\n", json!(events))).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "event": event }))
}

pub fn calendar_delete(sh: &crate::Shared, input: &Value) -> Result<Value, String> {
    let id = input_id(input)?;
    let path = sh.dir.join("calendar.json");
    let mut events: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<Value>>(&text).ok())
        .unwrap_or_default();
    let before = events.len();
    events.retain(|e| !same_id(e, &id));
    if events.len() == before {
        return Err("no event with that id".into());
    }
    std::fs::write(&path, format!("{}\n", json!(events))).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "id": id }))
}

// How many events one sync may take off the server. A person deleting an
// event or two and then syncing is the everyday case; a sync finding a crowd
// of them gone is nearly always a page that has not read its list yet, and
// the cost of being careful is a second sync.
const MOST_DELETIONS: usize = 3;

// Which of the events the server still holds are taken away, and which wait:
// those the list here no longer has, one by one. Two guards, both learned the
// hard way (2026-10-20: a list with nothing in it took sixteen of the person's
// own events off iCloud):
//
//   * nothing at all when the list here is empty — that is a page that has not
//     read its events yet, not a person who deleted them all;
//   * no more than MOST_DELETIONS in one sync — a crowd gone at once is a
//     mistake far more often than it is meant, and being careful costs a
//     second sync.
//
// What waits stays in the record, so the next sync looks at it again.
type SyncItem = (String, String, String, String);

fn gone_from_server(
    prev: &std::collections::HashMap<String, (String, String, String)>,
    local_ids: &std::collections::HashSet<String>,
    urls: &[String],
    local_empty: bool,
) -> (Vec<SyncItem>, Vec<SyncItem>) {
    let in_scope = |href: &str| urls.iter().any(|u| href.starts_with(u.trim_end_matches('/')));
    let (mut going, mut waiting) = (Vec::new(), Vec::new());
    for (id, (href, etag, sig)) in prev {
        if local_ids.contains(id) || href.is_empty() || !in_scope(href) {
            continue;
        }
        let item = (id.clone(), href.clone(), etag.clone(), sig.clone());
        if local_empty || going.len() >= MOST_DELETIONS {
            waiting.push(item);
        } else {
            going.push(item);
        }
    }
    (going, waiting)
}

async fn sync_one_account(app: AppHandle) -> Value {
    use base64::Engine as _;
    let sh = shared(&app);
    let accounts = calendar_accounts(&sh);
    let account_count = accounts.len();
    let active = calendar_active(&sh, &accounts);
    let Some(account) = accounts.iter().find(|a| a["id"].as_str() == Some(active.as_str())).cloned() else {
        return json!({ "ok": false, "error": "no account" });
    };
    let provider = account["provider"].as_str().unwrap_or("custom");
    let account_id = account["id"].as_str().unwrap_or("").to_string();
    let first_account = accounts.first().and_then(|a| a["id"].as_str()).unwrap_or("").to_string();
    let google = provider == "google";
    // Nextcloud's address is built from its three fields and Google's from a
    // calendar id, the same way 测试连接 built them: the sync read the raw
    // `url` alone, which is why a Nextcloud account (whose form never asks
    // for one) could not sync.
    let url = cal_account_url(&account);
    if url.is_empty() || (!google && cal_login(&account).is_empty()) {
        return json!({ "ok": false, "error": "no account" });
    }
    let proxy = match cal_proxy(&account) {
        Ok(p) => p.as_ref().map(proxy_url).unwrap_or_default(),
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let auth = if google {
        match google_token(&account) {
            Ok(token) => format!("Bearer {token}"),
            Err(e) => return json!({ "ok": false, "error": e }),
        }
    } else {
        let Some(password) = account_secret(account["id"].as_str().unwrap_or(""), CALDAV_SECRET) else {
            return json!({ "ok": false, "error": "no password saved" });
        };
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{}:{password}", cal_login(&account))))
    };
    let query = "<?xml version=\"1.0\" encoding=\"utf-8\"?><C:calendar-query xmlns:D=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><D:prop><D:getetag/><C:calendar-data/></D:prop><C:filter><C:comp-filter name=\"VCALENDAR\"/></C:filter></C:calendar-query>";
    // Ask the collection what calendars it holds: a calendars folder (iCloud,
    // Nextcloud) answers with its children; a single calendar answers with
    // itself, which is left as the one to pull. Works for both because the
    // answer is read rather than the URL guessed at.
    // The colour is asked for under both names it comes by: CalDAV's own and
    // Apple's (iCloud answers with the second, Google and Nextcloud with the
    // first). A server that has neither says so and the event is drawn in a
    // colour of its own.
    let propfind = "<?xml version=\"1.0\" encoding=\"utf-8\"?><d:propfind xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\" xmlns:a=\"http://apple.com/ns/ical/\"><d:prop><d:displayname/><d:resourcetype/><c:calendar-color/><a:calendar-color/></d:prop></d:propfind>";
    let base = url.split('/').take(3).collect::<Vec<_>>().join("/");
    // The calendars of this account, with the names and colours they gave.
    let mut calendars: Vec<CalCalendar> = vec![CalCalendar { href: url.clone(), ..Default::default() }];
    if let Some(xml) = caldav_call(&sh, "PROPFIND", &url, &auth, propfind, &proxy) {
        let found: Vec<CalCalendar> = caldav_calendars(&xml, &base)
            .into_iter()
            .filter(|c| c.href.trim_end_matches('/') != url.trim_end_matches('/'))
            .collect();
        if !found.is_empty() {
            calendars = found;
        }
    }
    let urls: Vec<String> = calendars.iter().map(|c| c.href.clone()).collect();
    let mut remote: std::collections::HashMap<String, CaldavItem> = std::collections::HashMap::new();
    // One child process per calendar, all at once: four calendars one after the
    // other spend most of the time starting PowerShell.
    let auth_ref = &auth;
    let proxy_ref = proxy.as_str();
    let sh_ref: &Shared = &sh;
    let reports: Vec<(String, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = urls
            .iter()
            .map(|calendar| {
                let calendar = calendar.clone();
                scope.spawn(move || caldav_call(sh_ref, "REPORT", &calendar, auth_ref, query, proxy_ref).map(|xml| (calendar, xml)))
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
    });
    for (calendar, xml) in reports {
        // The calendar this came out of, and the colour it gave itself: an
        // event is drawn in its calendar's colour, so two accounts' events
        // are told apart by where they are from.
        let cal = calendars.iter().find(|c| c.href == calendar);
        let cal_name = cal.map(|c| c.name.clone()).unwrap_or_default();
        let cal_color = cal.map(|c| c.color.clone()).unwrap_or_default();
        for mut item in caldav_items(&xml, &calendar) {
            if let Some(fields) = item.event.as_object_mut() {
                fields.insert("account".into(), json!(account_id));
                if !cal_name.is_empty() {
                    fields.insert("calendar".into(), json!(cal_name));
                }
                if !cal_color.is_empty() {
                    fields.insert("color".into(), json!(cal_color));
                }
            }
            remote.entry(item.id.clone()).or_insert(item);
        }
    }
    let local: Vec<Value> = std::fs::read_to_string(sh.dir.join("calendar.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    // What each account last saw: id → href, ETag, and the values it had here.
    // Kept per account, because one local calendar can hold events from more
    // than one: switching accounts must not push them all into the new one.
    // The key is the account's own id, not its address: two accounts may
    // point at the same server (a shared calendar seen twice, a second
    // login), and by the address they would have shared one record and
    // taken each other's events for their own.
    let map_path = sh.dir.join("calendar-sync.json");
    let mut accounts: serde_json::Map<String, Value> = std::fs::read_to_string(&map_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|stored| {
            stored["accounts"].as_object().cloned().or_else(|| {
                let account = stored["account"].as_str()?;
                let mut map = serde_json::Map::new();
                map.insert(account.to_string(), stored["items"].clone());
                Some(map)
            })
        })
        .unwrap_or_default();
    // The record of an account, under its own id. One kept under the
    // account's address (the way it was kept before) is taken over, and a
    // file with one record in it belongs to the one account there was.
    if !accounts.contains_key(&account_id) {
        // Every record kept under this account's address (the way it was
        // kept before: once for the collection, once for a calendar under
        // it) is this account's, and is taken over under its id. The old
        // keys go: left there they would be read as another account's, and
        // this account's own events would be taken for another's and not
        // pushed.
        let mine: Vec<String> = accounts
            .keys()
            .filter(|key| key.as_str() == url.as_str() || key.starts_with(&url))
            .cloned()
            .collect();
        if !mine.is_empty() {
            let mut items: Vec<Value> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for key in &mine {
                for item in accounts.get(key).and_then(|v| v.as_array().cloned()).unwrap_or_default() {
                    let id = item["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() || !seen.insert(id) {
                        continue;
                    }
                    items.push(item);
                }
                accounts.remove(key);
            }
            accounts.insert(account_id.clone(), Value::Array(items));
        } else if accounts.len() == 1 && account_count == 1 {
            // One record in the file and one account there: it is this
            // account's, kept under a name rather than an id.
            let only = accounts.keys().next().cloned();
            if let Some(key) = only {
                if let Some(items) = accounts.remove(&key) {
                    accounts.insert(account_id.clone(), items);
                }
            }
        }
    }
    let prev: std::collections::HashMap<String, (String, String, String)> = accounts
        .get(&account_id)
        .and_then(|v| v.as_array().cloned())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    let id = i["id"].as_str()?.to_string();
                    Some((id, (i["href"].as_str().unwrap_or("").to_string(), i["etag"].as_str().unwrap_or("").to_string(), i["sig"].as_str().unwrap_or("").to_string())))
                })
                .collect()
        })
        .unwrap_or_default();
    let foreign: std::collections::HashSet<String> = accounts
        .iter()
        .filter(|(account, _)| account.as_str() != account_id.as_str())
        .flat_map(|(_, items)| items.as_array().cloned().unwrap_or_default())
        .filter_map(|i| i["id"].as_str().map(String::from))
        .collect();
    let first = urls.first().cloned().unwrap_or_default();
    let local_ids: std::collections::HashSet<String> = local.iter().filter_map(|e| e["id"].as_str().map(String::from)).collect();
    let mut map_items: Vec<Value> = Vec::new();
    // What the server no longer holds, said with the account it was in: the
    // same event may be in two accounts (copied to both), and one account
    // losing it must not take the other's away.
    let mut removed: Vec<Value> = Vec::new();
    let (mut pushed, mut updated, mut deleted, mut failed) = (0, 0, 0, 0);
    // Only this account's events are pushed or compared: another account's
    // stay as they are, and an event made before accounts had ids belongs to
    // the first one.
    let mine = |e: &Value| e["account"].as_str().map_or(first_account == account_id, |a| a == account_id);
    for e in local.iter().filter(|e| mine(e)) {
        let id = e["id"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let sig = event_sig(e);
        // An event whose text came out broken is not written back: the text
        // is not what it is (a title of "????" is a title lost, not a title
        // changed), and putting it back would take the server's own text
        // with it. It waits for the next pull to bring the right one.
        if event_broken(e) {
            failed += 1;
            sh.log(&format!("calendar: {} is not written back: its text is broken (a title of ????); it waits for the next pull", item_href(&remote, &id)));
            map_items.push(json!({ "id": id, "href": item_href(&remote, &id), "etag": remote.get(&id).map(|i| i.etag.clone()).unwrap_or_default(), "sig": sig }));            continue;
        }
        if let Some(item) = remote.get(&id) {
            if event_sig(&item.event) == sig {
                map_items.push(json!({ "id": id, "href": item.href, "etag": item.etag, "sig": sig }));
            } else if let Some(raw) = caldav_get(&item.href, &auth, &proxy) {
                // Only the changed fields are replaced: whatever else the event
                // holds (alarms, attendees, colours) is left as the server had it.
                let body = ics_patch(&raw, e);
                let reply = caldav_send(
                    "PUT",
                    &item.href,
                    &auth,
                    &body,
                    "text/calendar; charset=utf-8",
                    if item.etag.is_empty() { None } else { Some(("If-Match", item.etag.as_str())) },
                    &proxy,
                );
                if reply.as_deref().is_some_and(|r| r.starts_with("ok|")) {
                    updated += 1;
                } else {
                    failed += 1;
                    sh.log(&format!("calendar: update of {} failed: {}", item.href, reply.as_deref().unwrap_or("no reply")));
                }
                let etag = reply.as_deref().and_then(|r| r.strip_prefix("ok|")).map(str::trim).filter(|t| !t.is_empty()).map(String::from).unwrap_or_else(|| item.etag.clone());
                map_items.push(json!({ "id": id, "href": item.href, "etag": etag, "sig": sig }));
            } else {
                failed += 1;
                sh.log(&format!("calendar: cannot read {} for an update", item.href));
                map_items.push(json!({ "id": id, "href": item.href, "etag": item.etag, "sig": sig }));
            }
        } else if let Some((href, etag, old_sig)) = prev.get(&id) {
            if old_sig == &sig {
                // Unchanged here but gone there: the server's deletion wins.
                removed.push(json!({ "account": account_id, "id": id }));
                continue;
            }
            // Changed here and gone there: put it back, same resource.
            let file: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
            let target = if href.is_empty() || file.is_empty() { format!("{}/{}.ics", first.trim_end_matches('/'), file) } else { href.clone() };
            let reply = caldav_send("PUT", &target, &auth, &ics_for(std::slice::from_ref(e)), "text/calendar; charset=utf-8", None, &proxy);
            if reply.as_deref().is_some_and(|r| r.starts_with("ok|")) {
                pushed += 1;
            } else {
                failed += 1;
            }
            let etag = reply.as_deref().and_then(|r| r.strip_prefix("ok|")).map(str::trim).filter(|t| !t.is_empty()).map(String::from).unwrap_or_else(|| etag.clone());
            map_items.push(json!({ "id": id, "href": target, "etag": etag, "sig": sig }));
        } else if foreign.contains(&id) {
            // Pulled from another calendar: it belongs there, not here.
            continue;
        } else {
            // New here: put into the first calendar, its own id as UID and file
            // name so the next pull answers with the same id.
            let file: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
            if file.is_empty() {
                continue;
            }
            let target = format!("{}/{}.ics", first.trim_end_matches('/'), file);
            let reply = caldav_send(
                "PUT",
                &target,
                &auth,
                &ics_for(std::slice::from_ref(e)),
                "text/calendar; charset=utf-8",
                Some(("If-None-Match", "*")),
                &proxy,
            );
            if reply.as_deref().is_some_and(|r| r.starts_with("ok|")) {
                pushed += 1;
                let etag = reply.as_deref().and_then(|r| r.strip_prefix("ok|")).map(str::trim).unwrap_or("").to_string();
                map_items.push(json!({ "id": id, "href": target, "etag": etag, "sig": sig }));
            } else {
                failed += 1;
            }
        }
    }
    // Gone here but it was on the server: delete that resource, unless it lives
    // outside the calendars this account points at now. Which ones go, and
    // which wait, is decided by gone_from_server, with the two guards that
    // were learned the hard way (see there).
    let (going, waiting) = gone_from_server(&prev, &local_ids, &urls, local.is_empty());
    for (id, href, etag, sig) in &going {
        if caldav_delete(href, &auth, &proxy) {
            deleted += 1;
        } else {
            failed += 1;
            map_items.push(json!({ "id": id, "href": href, "etag": etag, "sig": sig }));
        }
    }
    for (id, href, etag, sig) in &waiting {
        map_items.push(json!({ "id": id, "href": href, "etag": etag, "sig": sig }));
    }
    if !waiting.is_empty() {
        let why = if local.is_empty() {
            "the list here is empty, which is a page that has not read its events yet rather than a person who deleted them"
        } else {
            "only a few are taken away at a time"
        };
        sh.log(&format!(
            "calendar: {} events are gone from the list here but were on the server; nothing is deleted ({why}) — they are kept and looked at again next time",
            waiting.len()
        ));
    }
    accounts.insert(url.clone(), Value::Array(map_items));
    let map = json!({ "accounts": accounts });
    let _ = std::fs::write(&map_path, format!("{}\n", map));
    let mut events: Vec<Value> = remote.values().map(|i| i.event.clone()).collect();
    events.sort_by(|a, b| a["date"].as_str().cmp(&b["date"].as_str()));
    sh.log(&format!(
        "calendar: pulled {} events from {} calendars; pushed {pushed}, updated {updated}, deleted {deleted}, failed {failed}",
        events.len(),
        urls.len()
    ));
    json!({
        "ok": true,
        "events": events,
        "remote": events.len(),
        "calendars": urls.len(),
        "pushed": pushed,
        "updated": updated,
        "deleted": deleted,
        "removed": removed,
        "failed": failed,
    })
}

// Every account that is on, one after the other: the settings keep one active
// (where new events go), but sync reaches them all and merges what they hold.
#[tauri::command]
pub async fn calendar_sync(app: AppHandle) -> Value {
    let sh = shared(&app);
    let accounts = calendar_accounts(&sh);
    if accounts.is_empty() {
        return json!({ "ok": false, "error": "no account" });
    }
    let original = calendar_active(&sh, &accounts);
    let (mut pushed, mut updated, mut deleted, mut failed) = (0, 0, 0, 0);
    let mut calendars = 0usize;
    // Merged by the account the event is in and its own id: the same event
    // may be in two accounts (copied to both, as a shared calendar's), and
    // merging by the id alone would let one account's copy hide the other's.
    let mut events: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    let mut removed: Vec<Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for account in &accounts {
        if account["on"].as_bool() == Some(false) {
            continue;
        }
        let id = account["id"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        save_calendar_accounts(&sh, &accounts, &id);
        let got = sync_one_account(app.clone()).await;
        if got["ok"].as_bool() != Some(true) {
            errors.push(format!("{id}: {}", got["error"].as_str().unwrap_or("failed")));
            failed += 1;
            continue;
        }
        for event in got["events"].as_array().cloned().unwrap_or_default() {
            // The account and the id together say which event this is.
            if let Some(key) = cal_event_key(&event) {
                events.insert(key, event);
            }
        }
        pushed += got["pushed"].as_u64().unwrap_or(0);
        updated += got["updated"].as_u64().unwrap_or(0);
        deleted += got["deleted"].as_u64().unwrap_or(0);
        failed += got["failed"].as_u64().unwrap_or(0);
        calendars += got["calendars"].as_u64().unwrap_or(0) as usize;
        for gone in got["removed"].as_array().cloned().unwrap_or_default() {
            removed.push(gone);
        }
    }
    save_calendar_accounts(&sh, &accounts, &original);
    let mut list: Vec<Value> = events.into_values().collect();
    list.sort_by(|a, b| a["date"].as_str().cmp(&b["date"].as_str()));
    sh.log(&format!("calendar: {} accounts, {} events", accounts.len(), list.len()));
    json!({
        "ok": true,
        "events": list,
        "remote": list.len(),
        "calendars": calendars,
        "pushed": pushed,
        "updated": updated,
        "deleted": deleted,
        "removed": removed,
        "failed": failed,
        "errors": errors,
    })
}

// --- The calendar (data/calendar.json): events the person adds --------------
// Stored as the page sends them: [{ id, date: "YYYY-MM-DD", start, end, title,
// note, repeat: none|daily|weekly|monthly }]. The page expands the repeats.
// ICS files come in and go out as the same events (SUMMARY, DTSTART, DTEND,
// RRULE; text escaped as RFC 5545 has it).

fn ics_unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if (line.starts_with(' ') || line.starts_with('\t')) && !out.is_empty() {
            out.last_mut().unwrap().push_str(&line[1..]);
        } else {
            out.push(line.to_string());
        }
    }
    out
}

fn ics_text(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

fn ics_time(v: &str) -> Option<(String, String)> {
    let v = v.trim();
    let digits = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    if let Some((d, t)) = v.split_once('T') {
        let d = digits(d);
        let t = digits(t);
        if d.len() != 8 || t.len() < 4 {
            return None;
        }
        Some((format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]), format!("{}:{}", &t[0..2], &t[2..4])))
    } else {
        let d = digits(v);
        if d.len() != 8 {
            return None;
        }
        Some((format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]), String::new()))
    }
}

fn trigger_minutes(v: &str) -> Option<u64> {
    let v = v.trim().trim_start_matches('-').to_uppercase();
    if !v.starts_with('P') {
        return None;
    }
    let (mut total, mut num, mut any) = (0u64, 0u64, false);
    for c in v.chars().skip(1) {
        if c.is_ascii_digit() {
            num = num * 10 + (c as u64 - '0' as u64);
        } else {
            match c {
                'W' => total += num * 10_080,
                'D' => total += num * 1440,
                'H' => total += num * 60,
                'M' => total += num,
                'S' => total += num / 60,
                _ => {}
            }
            any |= matches!(c, 'W' | 'D' | 'H' | 'M' | 'S');
            num = 0;
        }
    }
    any.then_some(total)
}

fn ics_events(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut in_event = false;
    let mut trigger = String::new();
    let mut ev: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in ics_unfold(text) {
        if line == "BEGIN:VEVENT" {
            in_event = true;
            trigger.clear();
            ev.clear();
            continue;
        }
        if line == "END:VEVENT" {
            in_event = false;
            if let Some((date, start)) = ev.get("DTSTART").and_then(|v| ics_time(v)) {
                let end = ev.get("DTEND").and_then(|v| ics_time(v)).map(|(_, t)| t).unwrap_or_default();
                let repeat = ev
                    .get("RRULE")
                    .map(|r| r.to_uppercase())
                    .and_then(|r| {
                        if r.contains("FREQ=DAILY") {
                            Some("daily")
                        } else if r.contains("FREQ=WEEKLY") {
                            Some("weekly")
                        } else if r.contains("FREQ=MONTHLY") {
                            Some("monthly")
                        } else {
                            None
                        }
                    })
                    .unwrap_or("none");
                let id = ev.get("UID").cloned().filter(|u| !u.is_empty()).unwrap_or_else(|| format!("ics-{}", out.len()));
                // A trigger in whole days becomes the person's day setting;
                // anything finer stays in minutes.
                let (remind, remind_days) = match trigger_minutes(&trigger) {
                    Some(minutes) if minutes % 1440 == 0 => (0, Some(minutes / 1440)),
                    Some(minutes) => (minutes, None),
                    None => (0, None),
                };
                let mut event = json!({
                    "id": id,
                    "date": date,
                    "start": start,
                    "end": end,
                    "title": ev.get("SUMMARY").map(|s| ics_text(s)).unwrap_or_default(),
                    "note": ev.get("DESCRIPTION").map(|s| ics_text(s)).unwrap_or_default(),
                    "where": ev.get("LOCATION").map(|s| ics_text(s)).unwrap_or_default(),
                    "url": ev.get("URL").map(|s| s.trim().to_string()).unwrap_or_default(),
                    "repeat": repeat,
                    "remind": remind,
                });
                if let Some(days) = remind_days.filter(|d| *d <= 365) {
                    event["remindDays"] = json!(days);
                }
                out.push(event);
            }
            continue;
        }
        if !in_event {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.split(';').next().unwrap_or("").to_uppercase();
            // A calendar may carry an absolute alarm first and a relative
            // one after it; the relative one is the one we can keep.
            if key == "TRIGGER" && (trigger.is_empty() || (trigger_minutes(&trigger).is_none() && trigger_minutes(value).is_some())) {
                trigger = value.to_string();
            }
            ev.insert(key, value.to_string());
        }
    }
    out
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n")
}

fn ics_fields(e: &Value) -> Vec<String> {
    let date = e["date"].as_str().unwrap_or("");
    let start = e["start"].as_str().unwrap_or("");
    let end = e["end"].as_str().unwrap_or("");
    let stamp = |d: &str, t: &str| if t.is_empty() { d.replace('-', "") } else { format!("{}T{}00", d.replace('-', ""), t.replace(':', "")) };
    let mut out = Vec::new();
    if start.is_empty() {
        out.push(format!("DTSTART;VALUE=DATE:{}", date.replace('-', "")));
    } else {
        out.push(format!("DTSTART:{}", stamp(date, start)));
        if !end.is_empty() {
            out.push(format!("DTEND:{}", stamp(date, end)));
        }
    }
    out.push(format!("SUMMARY:{}", ics_escape(e["title"].as_str().unwrap_or(""))));
    if let Some(note) = e["note"].as_str().filter(|n| !n.is_empty()) {
        out.push(format!("DESCRIPTION:{}", ics_escape(note)));
    }
    if let Some(place) = e["where"].as_str().map(str::trim).filter(|p| !p.is_empty()) {
        out.push(format!("LOCATION:{}", ics_escape(place)));
    }
    if let Some(link) = e["url"].as_str().map(str::trim).filter(|l| !l.is_empty()) {
        out.push(format!("URL:{}", link.replace(['\r', '\n'], "")));
    }
    match e["repeat"].as_str() {
        Some("daily") => out.push("RRULE:FREQ=DAILY".to_string()),
        Some("weekly") => out.push("RRULE:FREQ=WEEKLY".to_string()),
        Some("monthly") => out.push("RRULE:FREQ=MONTHLY".to_string()),
        _ => {}
    }
    if let Some(days) = e["remindDays"].as_u64().filter(|n| *n <= 365) {
        out.push("BEGIN:VALARM".to_string());
        out.push("ACTION:DISPLAY".to_string());
        out.push("DESCRIPTION:Wakuwaku".to_string());
        if days == 0 {
            out.push("TRIGGER:PT0M".to_string());
        } else {
            out.push(format!("TRIGGER:-P{days}D"));
        }
        out.push("END:VALARM".to_string());
    } else if let Some(remind) = e["remind"].as_u64().filter(|n| *n > 0 && *n <= 10_080) {
        out.push("BEGIN:VALARM".to_string());
        out.push("ACTION:DISPLAY".to_string());
        out.push("DESCRIPTION:Wakuwaku".to_string());
        out.push(format!("TRIGGER:-PT{remind}M"));
        out.push("END:VALARM".to_string());
    }
    out
}

// The event with that UID gets the fields this app owns replaced; everything
// else in it (alarms, attendees, location, its other events) comes through
// untouched.
fn ics_patch(raw: &str, event: &Value) -> String {
    let id = event["id"].as_str().unwrap_or("");
    let owned = |line: &str| {
        let key = line.split([';', ':']).next().unwrap_or("");
        matches!(key, "UID" | "SUMMARY" | "DTSTART" | "DTEND" | "DESCRIPTION" | "LOCATION" | "URL" | "RRULE")
    };
    let lines = ics_unfold(raw);
    let mut out: Vec<String> = Vec::new();
    let mut done = false;
    let mut i = 0;
    while i < lines.len() {
        if lines[i] != "BEGIN:VEVENT" {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut uid = String::new();
        while j < lines.len() && lines[j] != "END:VEVENT" {
            if let Some(v) = lines[j].strip_prefix("UID:") {
                uid = v.trim().to_string();
            }
            j += 1;
        }
        let end = j.min(lines.len().saturating_sub(1));
        if !done && uid == id {
            out.push("BEGIN:VEVENT".to_string());
            out.push(format!("UID:{}", ics_escape(id)));
            out.extend(ics_fields(event));
            let mut in_alarm = false;
            for line in &lines[i + 1..end] {
                if line == "BEGIN:VALARM" {
                    in_alarm = true;
                    continue;
                }
                if line == "END:VALARM" {
                    in_alarm = false;
                    continue;
                }
                if !in_alarm && !owned(line) {
                    out.push(line.clone());
                }
            }
            if lines.get(end) == Some(&"END:VEVENT".to_string()) {
                out.push("END:VEVENT".to_string());
            }
            done = true;
        } else {
            for line in &lines[i..=end] {
                out.push(line.clone());
            }
        }
        i = end + 1;
    }
    if !done {
        return ics_for(std::slice::from_ref(event));
    }
    let mut text = out.join("\r\n");
    text.push_str("\r\n");
    text
}

fn ics_for(events: &[Value]) -> String {
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Wakuwaku//Calendar//EN\r\nCALSCALE:GREGORIAN\r\n");
    for e in events {
        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&format!("UID:{}\r\n", ics_escape(e["id"].as_str().unwrap_or(""))));
        for line in ics_fields(e) {
            out.push_str(&line);
            out.push_str("\r\n");
        }
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

#[tauri::command]
pub async fn calendar_get(app: AppHandle) -> Value {
    let sh = shared(&app);
    std::fs::read_to_string(sh.dir.join("calendar.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]))
}

#[tauri::command]
pub async fn calendar_set(app: AppHandle, events: Value) -> Value {
    let sh = shared(&app);
    let Value::Array(list) = events else { return json!({ "ok": false, "error": "events" }) };
    if list.len() > 2000 {
        return json!({ "ok": false, "error": "too many" });
    }
    let text = serde_json::to_string_pretty(&Value::Array(list)).unwrap_or_default();
    match std::fs::write(sh.dir.join("calendar.json"), format!("{text}\n")) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

// An ICS file written beside her data, from the events the page holds.
#[tauri::command]
pub async fn calendar_export(app: AppHandle, events: Value) -> Value {
    let sh = shared(&app);
    let list = events.as_array().cloned().unwrap_or_default();
    let path = sh.dir.join("calendar-export.ics");
    match std::fs::write(&path, ics_for(&list)) {
        Ok(()) => json!({ "ok": true, "path": path.to_string_lossy() }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

// Events read from the ICS file at that path (the page adds them).
#[tauri::command]
pub async fn calendar_import(path: String) -> Value {
    match std::fs::read_to_string(&path) {
        Ok(text) => json!({ "ok": true, "events": ics_events(&text) }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

#[tauri::command]
pub async fn calendar_import_text(text: String) -> Value {
    json!({ "ok": true, "events": ics_events(&text) })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn island_looks() {
        assert!(super::is_island_look(&json!(null)));
        assert!(super::is_island_look(&json!({})));
        assert!(super::is_island_look(&json!({ "bg": "#2C2c2c", "op": 62, "blur": 40, "edge": "ring", "bare": true })));
        assert!(!super::is_island_look(&json!({ "bg": "#2c2c2" })));
        assert!(!super::is_island_look(&json!({ "bg": "red" })));
        assert!(!super::is_island_look(&json!({ "op": 10 })));
        assert!(!super::is_island_look(&json!({ "blur": 61 })));
        assert!(!super::is_island_look(&json!({ "edge": "glow" })));
        assert!(!super::is_island_look(&json!({ "bare": 1 })));
        assert!(!super::is_island_look(&json!({ "ink": "light" })));
    }

    #[test]
    fn oauth_forms() {
        assert_eq!(super::url_encode("a b#c@d"), "a%20b%23c%40d");
        assert_eq!(super::url_encode("4/0Ab-c_d.e~f"), "4%2F0Ab-c_d.e~f");
        assert_eq!(super::form_body(&[("a", "1 2"), ("b", "x/y")]), "a=1%202&b=x%2Fy");
        assert_eq!(super::query_value("code=4%2F0Ab&state=x", "code"), Some("4/0Ab".into()));
        assert_eq!(super::query_value("error=access_denied", "error"), Some("access_denied".into()));
        assert_eq!(super::query_value("code=1", "state"), None);
    }

    // The address and the login name are worked out in one place, so 测试连接
    // and 同步 dial the same server. Each provider fills its form differently
    // and none of them may be left out of what it says.
    #[test]
    fn an_account_is_reached_where_its_form_says() {
        let url = super::cal_account_url;
        // Nextcloud: a server, a folder and a calendar, no path typed by hand.
        let nc = json!({ "provider": "nextcloud", "host": "https://cloud.example.com/", "user": "me@example.com", "calendar": "personal" });
        assert_eq!(url(&nc), "https://cloud.example.com/remote.php/dav/calendars/me@example.com/personal");
        // Without a calendar name, the calendars folder is the address, and the
        // sync finds the calendars under it.
        assert_eq!(url(&json!({ "provider": "nextcloud", "host": "cloud.example.com", "user": "me" })), "https://cloud.example.com/remote.php/dav/calendars/me");
        assert_eq!(url(&json!({ "provider": "nextcloud", "host": "cloud.example.com" })), "https://cloud.example.com");
        // A scheme of its own is left alone; http:// is the save's to refuse,
        // a password rides in the Basic header.
        assert_eq!(url(&json!({ "provider": "nextcloud", "host": "http://cloud.example.com", "user": "me" })), "http://cloud.example.com/remote.php/dav/calendars/me");
        // A host alone with no folder is not an address to sign in with.
        assert_eq!(url(&json!({ "provider": "nextcloud" })), "");
        // Google: a calendar id, which is usually the email.
        assert_eq!(url(&json!({ "provider": "google", "url": "me@gmail.com" })), "https://apidata.googleusercontent.com/caldav/v2/me@gmail.com/events");
        assert_eq!(url(&json!({ "provider": "google", "user": "me@gmail.com" })), "https://apidata.googleusercontent.com/caldav/v2/me@gmail.com/events");
        // '#' and a space are one id's characters, not the URL's.
        assert_eq!(url(&json!({ "provider": "google", "url": "a#b c" })), "https://apidata.googleusercontent.com/caldav/v2/a%23b%20c/events");
        assert_eq!(url(&json!({ "provider": "google" })), "");
        // A real URL in Google's field means the person knows where it is.
        assert_eq!(url(&json!({ "provider": "google", "url": "https://dav.test/c/" })), "https://dav.test/c");
        // The rest: the address as typed, without the trailing slash the sync
        // compares against; a bare host gains its scheme, an id never would.
        assert_eq!(url(&json!({ "provider": "icloud", "url": "https://pxx-caldav.icloud.com.cn/1/" })), "https://pxx-caldav.icloud.com.cn/1");
        assert_eq!(url(&json!({ "provider": "icloud", "url": "pxx-caldav.icloud.com.cn" })), "https://pxx-caldav.icloud.com.cn");
        assert_eq!(url(&json!({ "provider": "icloud", "url": "  " })), "");
    }

    #[test]
    fn the_login_name_is_not_always_the_folder() {
        let login = super::cal_login;
        assert_eq!(login(&json!({ "user": "me@example.com" })), "me@example.com");
        // Nextcloud's two fields: the folder under the address, and the name
        // signed in with, which are often not the same.
        assert_eq!(login(&json!({ "user": "me@example.com", "account": "jane" })), "jane");
        assert_eq!(login(&json!({ "user": "me@example.com", "account": "  " })), "me@example.com");
        assert_eq!(login(&json!({})), "");
    }

    // The account's own proxy: one address as typed, and the URL both sides
    // take. It was saved into the accounts file and read by nothing.
    #[test]
    fn an_accounts_proxy_is_read_where_it_is_typed() {
        let p = |v: serde_json::Value| super::cal_proxy_typed(&v).map(|p| p.map(|p| super::proxy_url(&p)));
        assert_eq!(p(json!({})), Ok(None));
        assert_eq!(p(json!({ "proxy": "  " })), Ok(None));
        // No scheme says HTTP, as it does everywhere else in her.
        assert_eq!(p(json!({ "proxy": " 127.0.0.1:7890 " })), Ok(Some("http://127.0.0.1:7890".into())));
        assert_eq!(p(json!({ "proxy": "http://Proxy.Example.com:3128" })), Ok(Some("http://proxy.example.com:3128".into())));
        // A scheme of its own says which kind; both reach PowerShell's
        // -Proxy and ureq alike.
        assert_eq!(p(json!({ "proxy": "socks5://127.0.0.1:10808" })), Ok(Some("socks5://127.0.0.1:10808".into())));
        assert_eq!(p(json!({ "proxy": "[::1]:1080" })), Ok(Some("http://[::1]:1080".into())));
        // An address that is not one is refused with the shape wanted, not
        // dropped: the field is read, so a typo says so.
        assert!(p(json!({ "proxy": "127.0.0.1" })).unwrap_err().contains("127.0.0.1:7890"));
        assert!(p(json!({ "proxy": "https://x:1" })).is_err());
    }

    // Two accounts may hold the same event (copied to both), so an event is
    // told apart by the account it is in and its own id, never by the id
    // alone: merging by the id alone let one account's copy hide the
    // other's, and one account losing an event took the other's away.
    #[test]
    fn an_event_is_told_apart_by_its_account_and_its_id() {
        let key = super::cal_event_key;
        let a = json!({ "id": "uid-1", "account": "acc-1" });
        let b = json!({ "id": "uid-1", "account": "acc-2" });
        let plain = json!({ "id": "uid-1" });
        assert_ne!(key(&a).unwrap(), key(&b).unwrap(), "the same id in two accounts is two events");
        assert_ne!(key(&a).unwrap(), key(&plain).unwrap(), "an event with no account is its own");
        assert_eq!(key(&a).unwrap(), key(&json!({ "id": "uid-1", "account": "acc-1" })).unwrap());
        // No id: no key.
        assert!(key(&json!({ "account": "acc-1" })).is_none());
        assert!(key(&json!({ "id": "", "account": "acc-1" })).is_none());
    }

    // A sync takes events off the server only when the list here says they are
    // gone: never when that list is empty, and only a few at a time. An empty
    // list is a page that has not read its events yet, not a person who
    // deleted them all; the day this was not held back, sixteen of a person's
    // own events went off iCloud (2026-10-20).
    #[test]
    fn a_sync_takes_off_only_what_the_list_says_is_gone() {
        use std::collections::{HashMap, HashSet};
        let url = "https://caldav.example.com/cal/".to_string();
        let urls = vec![url.clone()];
        let mut prev = HashMap::new();
        for id in ["a", "b", "c", "d", "e"] {
            prev.insert(id.to_string(), (format!("{url}{id}.ics"), "etag".to_string(), "sig".to_string()));
        }
        let ids = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<HashSet<_>>();
        // All five still here: nothing goes.
        let (going, waiting) = super::gone_from_server(&prev, &ids(&["a", "b", "c", "d", "e"]), &urls, false);
        assert!(going.is_empty() && waiting.is_empty());
        // Two gone: both go, nothing waits.
        let (going, waiting) = super::gone_from_server(&prev, &ids(&["a", "b", "c"]), &urls, false);
        assert_eq!(going.len(), 2, "the two the list no longer has");
        assert!(waiting.is_empty());
        // Five gone: a few go, the rest wait and are kept for the next sync.
        let (going, waiting) = super::gone_from_server(&prev, &ids(&[]), &urls, false);
        assert_eq!(going.len(), super::MOST_DELETIONS);
        assert_eq!(waiting.len(), 5 - super::MOST_DELETIONS);
        // An empty list is not five deletions: nothing goes at all.
        let (going, waiting) = super::gone_from_server(&prev, &ids(&[]), &urls, true);
        assert!(going.is_empty(), "an empty list deletes nothing");
        assert_eq!(waiting.len(), 5);
        // An event kept outside this account's calendars is not touched.
        let mut other = HashMap::new();
        other.insert("x".to_string(), ("https://dav.elsewhere/cal/x.ics".to_string(), "e".to_string(), "s".to_string()));
        let (going, waiting) = super::gone_from_server(&other, &ids(&[]), &urls, false);
        assert!(going.is_empty() && waiting.is_empty());
        // An event with no address to delete is not touched either.
        let mut nameless = HashMap::new();
        nameless.insert("y".to_string(), (String::new(), String::new(), String::new()));
        let (going, waiting) = super::gone_from_server(&nameless, &ids(&[]), &urls, false);
        assert!(going.is_empty() && waiting.is_empty());
    }

    // The calendars an account holds, with the names and colours they gave
    // themselves: an event is drawn in its calendar's colour, so two
    // accounts' events are told apart by where they are from.
    #[test]
    fn the_calendars_are_read_with_their_names_and_colours() {
        // The tag may come as <response> (DAV: as the default namespace) or
        // as <d:response>; both are read the same, and before this only the
        // first spelling was, so a server using the second gave nothing.
        for open in ["<response", "<d:response"] {
            let close = open.replace("<response", "</response");
            let xml = format!(
                r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:a="http://apple.com/ns/ical/">
{open}><d:href>/caldav/1/work/</d:href><d:propstat><d:prop><d:displayname>工作</d:displayname><d:resourcetype><d:collection/><c:calendar/></d:resourcetype><c:calendar-color>#FF3B30</c:calendar-color></d:prop></d:propstat>{close}>
{open}><d:href>/caldav/1/home/</d:href><d:propstat><d:prop><d:displayname>家里</d:displayname><d:resourcetype><d:collection/><c:calendar/></d:resourcetype><a:calendar-color>#34C759</a:calendar-color></d:prop></d:propstat>{close}>
{open}><d:href>/caldav/1/plain/</d:href><d:propstat><d:prop><d:displayname>没有颜色</d:displayname><d:resourcetype><d:collection/><c:calendar/></d:resourcetype></d:prop></d:propstat>{close}>
</d:multistatus>"#
            );
            let found = super::caldav_calendars(&xml, "https://caldav.example.com");
            assert_eq!(found.len(), 3, "{open}");
            assert_eq!(found[0].href.trim_end_matches('/'), "https://caldav.example.com/caldav/1/work");
            assert_eq!(found[0].name, "工作");
            assert_eq!(found[0].color, "FF3B30", "CalDAV's own calendar-color");
            // Apple's spelling of the same thing (iCloud answers with it).
            assert_eq!(found[1].color, "34C759");
            // A calendar that gives no colour is not refused over it.
            assert_eq!(found[2].name, "没有颜色");
            assert_eq!(found[2].color, "");
        }
        // A colour that is not one is not taken for a colour.
        let odd = super::caldav_calendars(
            r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:response><d:href>/c/</d:href><d:propstat><d:prop><d:displayname>x</d:displayname><d:resourcetype><d:collection/><c:calendar/></d:resourcetype><c:calendar-color>zzz</c:calendar-color></d:prop></d:propstat></d:response></d:multistatus>"#,
            "https://x.test",
        );
        assert_eq!(odd.len(), 1);
        assert_eq!(odd[0].color, "");
        // A collection that is not a calendar is not one of them.
        let plain = super::caldav_calendars(
            r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/c/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response></d:multistatus>"#,
            "https://x.test",
        );
        assert_eq!(plain.len(), 0);
    }

    // The events and the calendar they are in come back as they went: the
    // server's answer is UTF-8 (the PowerShell that fetched it writes the
    // bytes as they are, not as its console's code page can hold them), and
    // the names, the colours and the titles are read as they are.
    #[test]
    fn a_caldav_answer_is_read_as_it_came() {
        let propfind = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:a="http://apple.com/ns/ical/">
<d:response>
  <d:href>/caldav/1/work/</d:href>
  <d:propstat><d:prop>
    <d:displayname>工作日历</d:displayname>
    <d:resourcetype><d:collection/><c:calendar/></d:resourcetype>
    <c:calendar-color>#FF3B30</c:calendar-color>
  </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
</d:response>
</d:multistatus>"#;
        let calendars = super::caldav_calendars(propfind, "https://caldav.example.com");
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].name, "工作日历");
        assert_eq!(calendars[0].color, "FF3B30");

        let report = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
<d:response>
  <d:href>/caldav/1/work/u1.ics</d:href>
  <d:propstat><d:prop>
    <d:getetag>"u1-1"</d:getetag>
    <c:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:u1
DTSTART:20261020T090000
DTEND:20261020T100000
SUMMARY:周会讨论
END:VEVENT
END:VCALENDAR</c:calendar-data>
  </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
</d:response>
</d:multistatus>"#;
        let items = super::caldav_items(report, "https://caldav.example.com/caldav/1/work");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].event["title"], "周会讨论");
        assert_eq!(items[0].event["date"], "2026-10-20");
        assert_eq!(items[0].event["start"], "09:00");
        assert_eq!(items[0].event["end"], "10:00");
        assert_eq!(items[0].id, "u1");
    }

    // A text that came out broken is not written back to the server: it is a
    // title lost, not a title changed, and putting it back would take the
    // server's own text with it.
    #[test]
    fn a_broken_text_is_not_written_back() {
        let broken = |t: &str| super::event_broken(&json!({ "title": t, "note": "", "where": "" }));
        assert!(broken("????????"), "a run of question marks");
        assert!(broken("周会\u{FFFD}"), "a character that stands for one that could not be read");
        assert!(!broken("周会讨论"));
        assert!(!broken(""), "no title is empty, not broken");
        assert!(!broken("Why? Why not?"), "a question mark of its own is a question mark");
        // The note and the place count too.
        assert!(super::event_broken(&json!({ "title": "x", "note": "????", "where": "" })));
        assert!(super::event_broken(&json!({ "title": "x", "note": "", "where": "????" })));
    }

    #[test]
    fn where_and_link_survive_ics() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:u2\r\nDTSTART:20261027T090000\r\nSUMMARY:x\r\nLOCATION:Room 3\r\nURL:https://example.test/e/1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = super::ics_events(raw);
        assert_eq!(events[0]["where"], "Room 3");
        assert_eq!(events[0]["url"], "https://example.test/e/1");
        let out = super::ics_for(&events);
        assert!(out.contains("LOCATION:Room 3"));
        assert!(out.contains("URL:https://example.test/e/1"));
    }

    #[test]
    fn an_agent_event_is_checked() {
        let event = super::event_from_input(&json!({
            "title": "  评审  ",
            "date": "2026-10-19",
            "start": "9:05",
            "end": "10:00",
            "repeat": "weekly",
            "remind": 5,
            "where": "3 楼会议室",
            "url": "https://example.test/e/1",
        }))
        .unwrap();
        assert_eq!(event["title"], "评审");
        assert_eq!(event["start"], "09:05");
        assert_eq!(event["repeat"], "weekly");
        assert_eq!(event["remind"], 5);
        assert_eq!(event["where"], "3 楼会议室");
        assert_eq!(event["url"], "https://example.test/e/1");
        assert!(super::event_from_input(&json!({})).is_err());
        assert!(super::event_from_input(&json!({ "title": "x", "date": "19/10/2026" })).is_err());
        assert!(super::event_from_input(&json!({ "title": "x", "start": "10:00", "end": "09:00" })).is_err());
        assert!(super::event_from_input(&json!({ "title": "x", "repeat": "yearly" })).is_err());
        assert!(super::event_from_input(&json!({ "title": "x", "start": "25:00" })).is_err());
    }

    #[test]
    fn an_agent_update_keeps_the_rest() {
        let old = json!({ "id": 5, "date": "2026-10-19", "start": "09:00", "end": "10:00", "title": "旧", "note": "", "repeat": "none", "remind": 0 });
        let new = super::apply_update(&old, &json!({ "title": "新", "remind": 10 })).unwrap();
        assert_eq!(new["title"], "新");
        assert_eq!(new["start"], "09:00");
        assert_eq!(new["end"], "10:00");
        assert_eq!(new["remind"], 10);
        assert_eq!(new["id"], 5);
        assert!(super::apply_update(&old, &json!({ "start": "11:00", "end": "10:30" })).is_err());
        assert!(super::apply_update(&old, &json!({ "title": "  " })).is_err());
        assert_eq!(super::input_id(&json!({ "id": 7 })).unwrap(), "7");
        assert_eq!(super::input_id(&json!({ "id": "x" })).unwrap(), "x");
        assert!(super::input_id(&json!({})).is_err());
    }

    #[test]
    fn an_ics_event_keeps_what_we_do_not_own() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:u1\r\nDTSTART:20261027T090000\r\nDTEND:20261027T100000\r\nSUMMARY:old\r\nBEGIN:VALARM\r\nTRIGGER:-PT10M\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = json!({ "id": "u1", "date": "2026-10-27", "start": "10:30", "end": "11:30", "title": "new", "note": "", "repeat": "none", "remind": 10 });
        let patched = super::ics_patch(raw, &event);
        assert!(patched.contains("SUMMARY:new"));
        assert!(patched.contains("DTSTART:20261027T103000"));
        assert!(patched.contains("DTEND:20261027T113000"));
        assert!(!patched.contains("SUMMARY:old"));
        assert_eq!(patched.matches("BEGIN:VALARM").count(), 1);
        assert_eq!(patched.matches("TRIGGER:-PT10M").count(), 1);
        assert_eq!(patched.matches("UID:u1").count(), 1);
        assert_eq!(patched.matches("SUMMARY:").count(), 1);
    }

    #[test]
    fn pkce_challenges_are_sha256_of_the_verifier() {
        use base64::Engine as _;
        use sha2::Digest;
        let (verifier, challenge) = super::pkce_pair();
        assert!(verifier.len() >= 43);
        let expect = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()));
        assert_eq!(expect, challenge);
        assert_ne!(super::pkce_pair().0, verifier);
    }

    #[test]
    fn the_widget_shows_the_next_event_today() {
        let at = |h: i64, m: i64| h * 60 + m;
        let events = vec![
            json!({ "id": "a", "date": "2026-10-19", "start": "09:00", "end": "10:00", "title": "早会", "repeat": "none" }),
            json!({ "id": "b", "date": "2026-10-19", "start": "15:00", "end": "16:00", "title": "评审", "repeat": "none" }),
            json!({ "id": "c", "date": "2026-10-17", "start": "08:00", "end": "09:00", "title": "过去", "repeat": "none" }),
        ];
        let words = |minute| super::next_today_words(&events, 2026, 10, 19, minute, "zh");
        assert_eq!(words(at(12, 0)).as_deref(), Some("15:00 评审"));
        assert_eq!(words(at(9, 30)).as_deref(), Some("早会 · 进行中"));
        assert_eq!(words(at(11, 0)).as_deref(), Some("15:00 评审"));
        assert_eq!(words(at(17, 0)), None);
        let weekly = vec![json!({ "id": "w", "date": "2026-10-12", "start": "09:00", "end": "10:00", "title": "周会", "repeat": "weekly" })];
        assert_eq!(super::next_today_words(&weekly, 2026, 10, 19, at(8, 0), "zh").as_deref(), Some("09:00 周会"));
        assert_eq!(super::next_today_words(&weekly, 2026, 10, 20, at(8, 0), "zh"), None);
    }

    #[test]
    fn alarm_triggers_become_minutes() {
        assert_eq!(super::trigger_minutes("-PT10M"), Some(10));
        assert_eq!(super::trigger_minutes("-PT1H30M"), Some(90));
        assert_eq!(super::trigger_minutes("P1D"), Some(1440));
        assert_eq!(super::trigger_minutes("P1W"), Some(10080));
        assert_eq!(super::trigger_minutes("PT0M"), Some(0));
        assert_eq!(super::trigger_minutes("20261027T090000Z"), None);
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART:20261027T090000\r\nSUMMARY:x\r\nBEGIN:VALARM\r\nTRIGGER:-PT15M\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = super::ics_events(ics);
        assert_eq!(events[0]["remind"], 15);
        let back = super::ics_for(&events);
        assert!(back.contains("TRIGGER:-PT15M"));
        assert!(back.contains("BEGIN:VALARM"));
        // An absolute alarm before the relative one is skipped for the latter.
        let two = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a2\r\nDTSTART:20261027T090000\r\nSUMMARY:x\r\nBEGIN:VALARM\r\nTRIGGER:20261027T085000Z\r\nACTION:DISPLAY\r\nEND:VALARM\r\nBEGIN:VALARM\r\nTRIGGER:-PT30M\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = super::ics_events(two);
        assert_eq!(events[0]["remind"], 30);
    }

    // A trigger in whole days is the person's day setting, and the other way
    // around: an alarm at the event's own time is day 0, not "no reminder".
    #[test]
    fn day_alarms_become_days_and_back() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART:20261027T090000\r\nSUMMARY:x\r\nBEGIN:VALARM\r\nTRIGGER:-P2D\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = super::ics_events(raw);
        assert_eq!(events[0]["remindDays"], 2);
        assert_eq!(events[0]["remind"], 0);
        let back = super::ics_for(&events);
        assert!(back.contains("TRIGGER:-P2D"));

        let on_time = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:c\r\nDTSTART:20261027T090000\r\nSUMMARY:x\r\nBEGIN:VALARM\r\nTRIGGER:PT0M\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = super::ics_events(on_time);
        assert_eq!(events[0]["remindDays"], 0);
        assert!(super::ics_for(&events).contains("TRIGGER:PT0M"));
    }

    #[test]
    fn an_event_from_an_agent_takes_days_not_minutes() {
        let event = super::event_from_input(&json!({
            "title": "x",
            "date": "2026-10-27",
            "start": "09:00",
            "remind": 30,
            "remindDays": 3,
        }))
        .unwrap();
        assert_eq!(event["remindDays"], 3);
        assert_eq!(event["remind"], 0);

        let old = json!({ "id": 1, "date": "2026-10-27", "start": "09:00", "title": "x", "remind": 0, "remindDays": 3 });
        let changed = super::apply_update(&old, &json!({ "remind": 15 })).unwrap();
        assert_eq!(changed["remind"], 15);
        assert!(changed.get("remindDays").is_none());
    }

    // The day a reminder comes due: the event day itself (0), or that many
    // days before; an all-day event is at 09:00, as a phone's calendar has it.
    #[test]
    fn a_day_reminder_is_due_on_its_own_day_or_before() {
        let today = super::days_from_civil(2026, 10, 21);
        let midnight = today * 86_400_000;
        let hour = 3_600_000;
        let event = json!({ "date": "2026-10-28", "start": "09:00", "title": "x", "repeat": "none" });
        let due = super::next_occurrence(&event, today, midnight, midnight + 8 * hour, 7).unwrap();
        assert_eq!(due.date, "2026-10-28");
        assert_eq!(due.start, "09:00");
        let remind_at = due.at - 7 * 86_400_000;
        assert_eq!(remind_at, midnight + 9 * hour);
        assert!(midnight + 8 * hour < remind_at);
        assert!(midnight + 9 * hour >= remind_at);

        let all_day = json!({ "date": "2026-10-21", "start": "", "title": "x", "repeat": "none" });
        let due = super::next_occurrence(&all_day, today, midnight, midnight + 10 * hour, 0).unwrap();
        assert_eq!(due.date, "2026-10-21");
        assert_eq!(due.start, "");
        assert_eq!(due.at, midnight + 9 * hour);
        // Its day is over at midnight, not at 09:00 the next morning.
        assert!(super::next_occurrence(&all_day, today + 1, midnight + 86_400_000, midnight + 32 * hour, 0).is_none());

        let timed = json!({ "date": "2026-10-21", "start": "09:00", "title": "x", "repeat": "none" });
        assert!(super::next_occurrence(&timed, today, midnight, midnight + 10 * hour + 30 * 60_000, 0).is_none());
    }

    #[test]
    fn a_repeat_finds_the_occurrence_the_reminder_belongs_to() {
        let today = super::days_from_civil(2026, 10, 21);
        let midnight = today * 86_400_000;
        let weekly = json!({ "date": "2026-10-19", "start": "09:00", "title": "x", "repeat": "weekly" });
        let due = super::next_occurrence(&weekly, today, midnight, midnight + 10 * 3_600_000, 7).unwrap();
        assert_eq!(due.date, "2026-10-26");

        let monthly = json!({ "date": "2026-01-31", "start": "09:00", "title": "x", "repeat": "monthly" });
        let feb = super::days_from_civil(2026, 2, 1);
        let due = super::next_occurrence(&monthly, feb, feb * 86_400_000, feb * 86_400_000, 0).unwrap();
        assert_eq!(due.date, "2026-03-31");
    }

    #[test]
    fn repeats_with_the_reminders() {
        let daily = json!({ "date": "2026-10-19", "repeat": "daily" });
        assert!(super::occurs_on(&daily, 2026, 10, 19));
        assert!(super::occurs_on(&daily, 2026, 10, 20));
        assert!(!super::occurs_on(&daily, 2026, 10, 18));
        let weekly = json!({ "date": "2026-10-19", "repeat": "weekly" });
        assert!(super::occurs_on(&weekly, 2026, 10, 26));
        assert!(!super::occurs_on(&weekly, 2026, 10, 25));
        let monthly = json!({ "date": "2026-10-19", "repeat": "monthly" });
        assert!(super::occurs_on(&monthly, 2026, 11, 19));
        assert!(!super::occurs_on(&monthly, 2026, 11, 20));
        let once = json!({ "date": "2026-10-19", "repeat": "none" });
        assert!(super::occurs_on(&once, 2026, 10, 19));
        assert!(!super::occurs_on(&once, 2026, 10, 20));
    }
}

#[cfg(test)]
mod layout_tests {
    use serde_json::json;

    #[test]
    fn taskbar_layouts() {
        assert!(super::is_taskbar_layout(&json!({ "left": ["her"], "center": ["start", "apps"], "right": ["monitor", "tray", "keys", "quick", "clock"] })));
        // The sessions' tags are no part any more (data.rs drops them as it reads).
        assert!(!super::is_taskbar_layout(&json!({ "center": ["start", "apps", "tags"] })));
        assert!(super::is_taskbar_layout(&json!({ "left": ["start", "apps"], "right": ["clock", "her"] })));
        assert!(!super::is_taskbar_layout(&json!({ "left": ["her", "her"] })));
        assert!(!super::is_taskbar_layout(&json!({ "left": ["her"], "right": ["her"] })));
        assert!(!super::is_taskbar_layout(&json!({ "top": ["her"] })));
        assert!(!super::is_taskbar_layout(&json!({ "left": ["desk"] })));
        assert!(!super::is_taskbar_layout(&json!({ "left": "her" })));
        assert!(!super::is_taskbar_layout(&json!(["her"])));
    }

    #[test]
    fn widgets_on_the_taskbar() {
        let ok = |w| super::is_taskbar_layout(&json!({ "right": [w, "clock"] }));
        assert!(ok(json!({ "id": "wa1", "type": "widget", "of": "today", "size": "M" })));
        assert!(ok(json!({ "id": "wa1", "type": "widget", "of": "inbox-x2", "name": "me@example.com", "icon": "mail" })));
        assert!(ok(json!({ "id": "wa1", "type": "plugin", "of": "weather", "size": "S", "icon": "weather" })));
        assert!(ok(json!({ "id": "wa1", "type": "flex" })));
        assert!(ok(json!({ "id": "wa1", "type": "sep" })));
        // Not a plugin she runs; no widget's id; a spacer of something.
        assert!(!ok(json!({ "id": "wa1", "type": "plugin", "of": "nope" })));
        assert!(!ok(json!({ "id": "wa1", "type": "widget" })));
        assert!(!ok(json!({ "id": "wa1", "type": "widget", "of": "Not An Id" })));
        assert!(!ok(json!({ "id": "wa1", "type": "gap", "of": "today" })));
        // Its id, size, icon, name; nothing else.
        assert!(!ok(json!({ "id": "start", "type": "flex" })));
        assert!(!ok(json!({ "id": "w", "type": "flex" })));
        assert!(!ok(json!({ "id": "wa1", "type": "widget", "of": "today", "size": "L" })));
        assert!(!ok(json!({ "id": "wa1", "type": "widget", "of": "today", "icon": "nope" })));
        assert!(!ok(json!({ "id": "wa1", "type": "widget", "of": "today", "name": "x".repeat(41) })));
        assert!(!ok(json!({ "id": "wa1", "type": "flex", "colour": "#fff" })));
        assert!(!ok(json!({ "id": "wa1", "type": "clock" })));
        // Each once, 32 at most.
        assert!(!super::is_taskbar_layout(&json!({ "left": [{ "id": "wa1", "type": "sep" }], "right": [{ "id": "wa1", "type": "sep" }] })));
        let many: Vec<_> = (0..33).map(|i| json!({ "id": format!("w{i}"), "type": "gap" })).collect();
        assert!(!super::is_taskbar_layout(&json!({ "right": many })));
        assert!(super::is_taskbar_layout(&json!({ "right": many[..32] })));
    }

    // Several in one place: a carousel or a stack of 2 to 8 widgets' and
    // plugins', each as one put there alone, none twice; no size of its own.
    #[test]
    fn several_widgets_in_one_place() {
        let ok = |w| super::is_taskbar_layout(&json!({ "right": [w, "clock"] }));
        let today = json!({ "type": "widget", "of": "today", "icon": "today" });
        let stock = json!({ "type": "plugin", "of": "stock", "name": "股票" });
        let mail = json!({ "type": "widget", "of": "inbox-a1", "name": "me@example.com", "icon": "mail" });
        assert!(ok(json!({ "id": "wc1", "type": "carousel", "items": [today, stock] })));
        assert!(ok(json!({ "id": "wc1", "type": "stack", "items": [today, stock, mail] })));
        // One alone, more than eight, the same one twice: not.
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today] })));
        let nine: Vec<_> = (0..9).map(|i| json!({ "type": "widget", "of": format!("w{i}") })).collect();
        assert!(!ok(json!({ "id": "wc1", "type": "carousel", "items": nine })));
        assert!(ok(json!({ "id": "wc1", "type": "carousel", "items": nine[..8] })));
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today, today] })));
        // A member as a widget alone is, nothing else: no spacer, no size, no id.
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today, { "type": "sep" }] })));
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today, { "type": "plugin", "of": "nope" }] })));
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today, { "type": "widget", "of": "tokens", "size": "M" }] })));
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "items": [today, { "id": "wd", "type": "widget", "of": "tokens" }] })));
        // The group: items and no of, no size; items only on a group.
        assert!(!ok(json!({ "id": "wc1", "type": "stack" })));
        assert!(!ok(json!({ "id": "wc1", "type": "stack", "of": "today", "items": [today, stock] })));
        assert!(!ok(json!({ "id": "wc1", "type": "carousel", "size": "M", "items": [today, stock] })));
        assert!(!ok(json!({ "id": "wc1", "type": "widget", "of": "tokens", "items": [today, stock] })));
        // What it holds is on the taskbar.
        let layout = json!({ "right": [{ "id": "wc1", "type": "stack", "items": [today, stock] }, "clock"] });
        assert!(super::is_on_taskbar(&layout, "today") && super::is_on_taskbar(&layout, "stock-nvda"));
        assert!(!super::is_on_taskbar(&layout, "tokens"));
    }

    #[test]
    fn what_the_taskbar_shows() {
        let layout = json!({ "left": ["her"], "right": [{ "id": "wa", "type": "widget", "of": "tokens" }, { "id": "wb", "type": "plugin", "of": "stock" }, "clock"] });
        assert!(super::is_on_taskbar(&layout, "tokens"));
        assert!(super::is_on_taskbar(&layout, "stock-aapl"));
        assert!(!super::is_on_taskbar(&layout, "today"));
        assert!(!super::is_on_taskbar(&layout, "weather"));
        assert!(!super::is_on_taskbar(&json!(null), "tokens"));
    }

    // The desktop's widgets, as deskgrid.js isLayout has them.
    #[test]
    fn desk_layouts() {
        let ok = |v: serde_json::Value| super::is_desk_layout(&v);
        assert!(ok(json!([])));
        assert!(ok(json!([
            { "id": "wa", "type": "widget", "of": "weather", "size": "m", "d": "d0a1b2c3d4e5f", "at": { "d0a1b2c3d4e5f": [3, 0], "d1": [0, 2] } },
            { "id": "wb", "type": "plugin", "of": "stock", "size": "l", "name": "股票", "icon": "chart" },
            { "id": "ws", "type": "stack", "size": "s", "items": [{ "type": "widget", "of": "monitor" }, { "type": "widget", "of": "countdown", "name": "倒计时" }] }
        ])));
        let bad = [
            json!({}),
            json!([{ "id": "wa", "type": "widget", "of": "weather", "size": "S" }]),
            json!([{ "id": "wa", "type": "widget", "of": "weather" }]),
            json!([{ "id": "Wa", "type": "widget", "of": "weather", "size": "s" }]),
            json!([{ "id": "wa", "type": "widget", "of": "weather", "size": "s", "d": "D1" }]),
            json!([{ "id": "wa", "type": "widget", "of": "weather", "size": "s", "at": { "d1": [1, 256] } }]),
            json!([{ "id": "wa", "type": "widget", "of": "weather", "size": "s", "at": { "d1": [1] } }]),
            json!([{ "id": "wa", "type": "widget", "of": "weather", "size": "s", "x": 1 }]),
            json!([{ "id": "wa", "type": "plugin", "of": "nope", "size": "s" }]),
            json!([{ "id": "wa", "type": "carousel", "size": "s", "items": [{ "type": "widget", "of": "a" }, { "type": "widget", "of": "b" }] }]),
            json!([{ "id": "ws", "type": "stack", "size": "s", "items": [{ "type": "widget", "of": "a" }] }]),
            json!([{ "id": "ws", "type": "stack", "size": "s", "items": [{ "type": "widget", "of": "a" }, { "type": "widget", "of": "a" }] }]),
            json!([{ "id": "ws", "type": "stack", "size": "s", "name": "x", "items": [{ "type": "widget", "of": "a" }, { "type": "widget", "of": "b" }] }]),
            json!([{ "id": "wa", "type": "widget", "of": "a", "size": "s" }, { "id": "wa", "type": "widget", "of": "b", "size": "s" }]),
            json!([{ "id": "wa", "type": "widget", "of": "a", "size": "s" }, { "id": "ws", "type": "stack", "size": "s", "items": [{ "type": "widget", "of": "a" }, { "type": "widget", "of": "b" }] }]),
        ];
        for b in bad {
            assert!(!ok(b.clone()), "{b}");
        }
        assert!(!ok(serde_json::Value::Array((0..33).map(|i| json!({ "id": format!("w{i}"), "type": "widget", "of": format!("x{i}"), "size": "s" })).collect())));
        // Boards: their tiles on their grid of 8 by 4, none over another.
        assert!(ok(json!([{ "id": "wb", "type": "board", "name": "行情", "folded": true, "d": "d1", "at": { "d1": [1, 1] }, "items": [
            { "type": "widget", "of": "monitor", "size": "m", "at": [0, 0] },
            { "type": "plugin", "of": "stock", "size": "l", "at": [4, 0] },
            { "type": "widget", "of": "today", "size": "s", "at": [0, 2], "name": "今天" }
        ] }, { "id": "wa", "type": "widget", "of": "weather", "size": "s" }])));
        assert!(ok(json!([{ "id": "wb", "type": "board", "items": [] }])));
        let tiles = |items: serde_json::Value| json!([{ "id": "wb", "type": "board", "items": items }]);
        let bad_boards = [
            tiles(json!([{ "type": "widget", "of": "a", "size": "s", "at": [0, 0] }, { "type": "widget", "of": "b", "size": "s", "at": [1, 1] }])),
            tiles(json!([{ "type": "widget", "of": "a", "size": "m", "at": [5, 0] }])),
            tiles(json!([{ "type": "widget", "of": "a", "size": "l", "at": [0, 1] }])),
            tiles(json!([{ "type": "widget", "of": "a", "size": "s" }])),
            tiles(json!([{ "type": "widget", "of": "a", "size": "s", "at": [0, 0], "x": 1 }])),
            tiles(json!([{ "type": "widget", "of": "a", "size": "s", "at": [0, 0] }, { "type": "widget", "of": "a", "size": "s", "at": [2, 0] }])),
            tiles(json!((0..17).map(|i| json!({ "type": "widget", "of": format!("t{i}"), "size": "s", "at": [0, 0] })).collect::<Vec<_>>())),
            json!([{ "id": "wb", "type": "board", "items": [], "name": "一二三四五六七八九十一二三四五六七" }]),
            json!([{ "id": "wb", "type": "board", "items": [], "size": "s" }]),
            json!([{ "id": "wb", "type": "board", "items": [], "folded": 1 }]),
            json!([{ "id": "wb", "type": "board" }]),
            json!([{ "id": "wb", "type": "board", "items": [{ "type": "widget", "of": "a", "size": "s", "at": [0, 0] }] }, { "id": "wa", "type": "widget", "of": "a", "size": "s" }]),
        ];
        for b in bad_boards {
            assert!(!ok(b.clone()), "{b}");
        }
        // What it shows.
        let layout = json!([{ "id": "wa", "type": "widget", "of": "monitor", "size": "m" }, { "id": "ws", "type": "stack", "size": "s", "items": [{ "type": "plugin", "of": "stock" }, { "type": "widget", "of": "today" }] }]);
        assert!(super::is_on_desk(&layout, "monitor"));
        assert!(super::is_on_desk(&layout, "stock-aapl"));
        assert!(super::is_on_desk(&layout, "today"));
        assert!(!super::is_on_desk(&layout, "weather"));
        assert!(!super::is_on_desk(&json!(null), "monitor"));
        // In a board.
        assert!(super::is_on_desk(&json!([{ "id": "wb", "type": "board", "items": [{ "type": "widget", "of": "weather", "size": "s", "at": [0, 0] }] }]), "weather"));
    }

    // Boards kept as combos, as deskgrid.js isBoardSets has them (the same
    // edges as test/deskboard.test.js).
    #[test]
    fn board_sets() {
        let ok = |v: serde_json::Value| super::is_board_sets(&v);
        let t1 = json!([{ "type": "widget", "of": "today", "size": "m", "at": [0, 0] }, { "type": "plugin", "of": "weather", "size": "s", "at": [4, 0] }]);
        let set = |id: &str, name: &str, items: &serde_json::Value| json!({ "id": id, "name": name, "items": items, "saved": 1 });
        let one = json!([t1[1].clone()]);
        assert!(ok(json!([set("s1", "写代码", &t1), set("s2", "摸鱼", &one)])));
        assert!(ok(json!([])));
        assert!(ok(json!([{ "id": "s0a1b2c3d4e5f6g7", "name": "一二三四五六七八九十一二三四五六", "items": t1, "saved": 9_007_199_254_740_991u64 }])));
        let named = |name: &str| ok(json!([set("s1", name, &t1)]));
        assert!(named("行 情"));
        for bad in ["", " 前后有空格 ", "前有\u{3000}", "\u{feff}前", "一二三四五六七八九十一二三四五六七", "换\n行", "删\u{7f}"] {
            assert!(!named(bad), "{bad:?}");
        }
        let with = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut s = set("s1", "写代码", &t1);
            f(&mut s);
            ok(json!([s]))
        };
        let bad: [&dyn Fn(&mut serde_json::Value); 13] = [
            &|s| s["id"] = json!("x1"),
            &|s| s["id"] = json!("s"),
            &|s| s["id"] = json!("sA1"),
            &|s| s["id"] = json!("s0123456789abcdef"),
            &|s| s["items"] = json!([]),
            &|s| s["items"][0]["name"] = json!("今天"),
            &|s| s["items"][1]["icon"] = json!("weather"),
            &|s| s["items"][1]["at"] = json!([2, 0]),
            &|s| s["items"][1]["of"] = json!("today"),
            &|s| s["extra"] = json!(1),
            &|s| s["saved"] = json!(-1),
            &|s| s["saved"] = json!(1.5),
            &|s| s["saved"] = json!(9_007_199_254_740_992u64),
        ];
        for (i, f) in bad.iter().enumerate() {
            assert!(!with(*f), "case {i}");
        }
        assert!(!with(&|s| {
            s.as_object_mut().unwrap().remove("saved");
        }));
        // Ids and names once; twenty at most.
        assert!(!ok(json!([set("s1", "写代码", &t1), set("s1", "摸鱼", &t1)])));
        assert!(!ok(json!([set("s1", "写代码", &t1), set("s2", "写代码", &t1)])));
        let many = |n: usize| serde_json::Value::Array((0..n).map(|i| set(&format!("s{i}"), &format!("组合 {i}"), &t1)).collect());
        assert!(ok(many(20)));
        assert!(!ok(many(21)));
        assert!(!ok(json!({})));
        assert!(!ok(json!([null])));
    }

    // Her capsule, as capsule.js storedOf and its patches have it.
    #[test]
    fn capsules() {
        let ok = |v: serde_json::Value| super::is_capsule(&v);
        assert!(ok(json!({ "members": [{ "key": "widget:monitor", "mode": "pin" }, { "key": "plugin:weather", "mode": "turn" }, { "key": "widget:mail-a_1", "mode": "open" }], "spin": 8 })));
        assert!(ok(json!({ "members": [] })));
        // A turn's length alone (capsule.js spinPatch before any order), or nothing.
        assert!(ok(json!({ "spin": 0 })));
        assert!(ok(json!({})));
        let long = "a".repeat(40);
        assert!(ok(json!({ "members": [{ "key": format!("widget:{long}"), "mode": "turn" }] })));
        let bad = [
            json!(null),
            json!([]),
            json!({ "members": [{ "key": "widget:a", "mode": "x" }] }),
            json!({ "members": [{ "key": "bad", "mode": "turn" }] }),
            json!({ "members": [{ "key": "widget:", "mode": "turn" }] }),
            json!({ "members": [{ "key": "script:a", "mode": "turn" }] }),
            json!({ "members": [{ "key": "widget:A", "mode": "turn" }] }),
            json!({ "members": [{ "key": "widget:a.b", "mode": "turn" }] }),
            json!({ "members": [{ "key": format!("widget:{long}a"), "mode": "turn" }] }),
            json!({ "members": [{ "key": "widget:b", "mode": "turn" }, { "key": "widget:b", "mode": "pin" }] }),
            json!({ "members": [{ "key": "widget:a", "mode": "turn", "x": 1 }] }),
            json!({ "members": [{ "key": "widget:a" }] }),
            json!({ "members": "widget:a" }),
            json!({ "spin": 7 }),
            json!({ "spin": "8" }),
            json!({ "spin": 8, "order": [] }),
        ];
        for b in bad {
            assert!(!ok(b.clone()), "{b}");
        }
        let many = |n: usize| json!({ "members": (0..n).map(|i| json!({ "key": format!("widget:w{i}"), "mode": "turn" })).collect::<Vec<_>>() });
        assert!(ok(many(64)));
        assert!(!ok(many(65)));
    }

    // A widget in one place: on the desktop, or on the taskbar while that is
    // her home, it is out of the island (not pinned there either).
    #[test]
    fn a_widget_in_one_place() {
        let desk = json!([{ "id": "wa", "type": "widget", "of": "monitor", "size": "m" }]);
        let bar = json!({ "left": ["her"], "center": ["start", "apps"], "right": [{ "id": "w1", "type": "widget", "of": "today", "size": "S" }, "tray", "clock"] });
        assert_eq!(super::is_placed_out(&desk, &bar, false, "monitor"), cfg!(windows));
        assert!(super::is_placed_out(&json!([]), &bar, true, "today"));
        // The taskbar not her home: its layout is not there to show it.
        assert!(!super::is_placed_out(&json!([]), &bar, false, "today"));
        assert!(!super::is_placed_out(&desk, &bar, true, "weather"));
    }
}
