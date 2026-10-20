// The Wakuwaku MCP server (`wakuwaku.exe --mcp`): JSON-RPC over stdio, which
// an agent starts itself (Claude Code, Trae, WorkBuddy: their MCP configs name
// this exe and the flag). The tools are answered from her own HTTP API on
// 127.0.0.1:<port> (server.rs): what she follows, the mailboxes' newest
// letters, the dev servers she found, one started again, a line on her island.
// She may be closed: every tool then says she is not answering, and starting
// her is the person's call (her hooks and login do that).
//
// Protocol: MCP 2025-06-18 over newline-delimited JSON-RPC, the tools only.

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::time::Duration;

pub fn serve(port: u16) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(line) else { continue };
        // A notification (no id) is taken and not answered.
        let Some(id) = msg.get("id").cloned() else { continue };
        let method = msg["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "wakuwaku", "version": env!("CARGO_PKG_VERSION") },
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => call(port, &msg["params"]),
            other => Err(format!("unknown method {other}")),
        };
        let reply = match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(message) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32603, "message": message } }),
        };
        let _ = writeln!(out, "{}", serde_json::to_string(&reply).unwrap_or_default());
        let _ = out.flush();
    }
    Ok(())
}

fn tools() -> Value {
    json!([
        {
            "name": "wakuwaku_sessions",
            "description": "What Wakuwaku is following now: every agent session (Claude Code, Codex, Trae, ...), its project, mood, step and last reply.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "wakuwaku_notify",
            "description": "Put a line on Wakuwaku's island, telling the person something from this agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "label": { "type": "string", "description": "The line's name, 40 characters at most" },
                    "value": { "type": "string", "description": "What it says, 24 characters at most" },
                    "text": { "type": "string", "description": "A sentence under it, 60 characters at most" },
                },
                "required": ["label", "value"],
            },
        },
        {
            "name": "wakuwaku_mail",
            "description": "The mailboxes Wakuwaku watches and their newest letters (from, subject, when, unread).",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "wakuwaku_mail_read",
            "description": "Read one letter Wakuwaku keeps: its subject, from, to, date and text.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "The mailbox's id, as wakuwaku_mail gives it" },
                    "uid": { "type": "number", "description": "The letter's uid in that folder" },
                    "folder": { "type": "string", "description": "inbox (default), sent, drafts, ..." },
                },
                "required": ["account", "uid"],
            },
        },
        {
            "name": "wakuwaku_mail_send",
            "description": "Send a letter from one of her mailboxes (the person must have set the mailbox up).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "The mailbox to send from, as wakuwaku_mail gives it" },
                    "to": { "type": "string", "description": "Who, as one types it: \"Zhang San <zs@example.test>, li@example.test\"" },
                    "cc": { "type": "string" },
                    "bcc": { "type": "string" },
                    "subject": { "type": "string" },
                    "text": { "type": "string", "description": "The letter's words" },
                },
                "required": ["account", "to", "subject", "text"],
            },
        },
        {
            "name": "wakuwaku_devservers",
            "description": "The local dev servers Wakuwaku found: project, port, command, and whether each is up.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "wakuwaku_devserver_start",
            "description": "Start one dev server kept as stopped, with the command it was seen with.",
            "inputSchema": {
                "type": "object",
                "properties": { "port": { "type": "number", "description": "The port it listens on" } },
                "required": ["port"],
            },
        },
        {
            "name": "wakuwaku_calendar",
            "description": "The events on one day of Wakuwaku's calendar (today by default), repeats expanded: id, date, start, end, title, note, repeat and remind.",
            "inputSchema": {
                "type": "object",
                "properties": { "date": { "type": "string", "description": "YYYY-MM-DD; today when empty" } },
            },
        },
        {
            "name": "wakuwaku_calendar_add",
            "description": "Add an event to Wakuwaku's calendar. The person sees it in the Calendar page, and the next sync sends it to their CalDAV account.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "date": { "type": "string", "description": "YYYY-MM-DD; today when empty" },
                    "start": { "type": "string", "description": "HH:MM; empty for an all-day event" },
                    "end": { "type": "string", "description": "HH:MM" },
                    "note": { "type": "string" },
                    "where": { "type": "string", "description": "Where it is" },
                    "url": { "type": "string", "description": "A link (http/https)" },
                    "repeat": { "type": "string", "description": "none (default), daily, weekly, monthly" },
                    "remind": { "type": "number", "description": "minutes before; 0 for none" },
                },
                "required": ["title"],
            },
        },
        {
            "name": "wakuwaku_calendar_update",
            "description": "Change an event in Wakuwaku's calendar by id; only the fields given change. The next sync writes it back.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The event's id, as wakuwaku_calendar gives it" },
                    "title": { "type": "string" },
                    "date": { "type": "string", "description": "YYYY-MM-DD" },
                    "start": { "type": "string", "description": "HH:MM; empty for an all-day event" },
                    "end": { "type": "string", "description": "HH:MM" },
                    "note": { "type": "string" },
                    "where": { "type": "string", "description": "Where it is" },
                    "url": { "type": "string", "description": "A link (http/https)" },
                    "repeat": { "type": "string", "description": "none, daily, weekly, monthly" },
                    "remind": { "type": "number", "description": "minutes before; 0 for none" },
                },
                "required": ["id"],
            },
        },
        {
            "name": "wakuwaku_calendar_delete",
            "description": "Remove an event from Wakuwaku's calendar by id; the next sync deletes it from the server too.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "The event's id, as wakuwaku_calendar gives it" } },
                "required": ["id"],
            },
        },
        {
            "name": "wakuwaku_health",
            "description": "Whether Wakuwaku is running, and what it shows (the top session, its mood).",
            "inputSchema": { "type": "object", "properties": {} },
        },
    ])
}

fn call(port: u16, params: &Value) -> Result<Value, String> {
    let name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let got = match name {
        "wakuwaku_sessions" => get(port, "/sessions")?,
        "wakuwaku_health" => get(port, "/health")?,
        "wakuwaku_mail" => get(port, "/mail")?,
        "wakuwaku_mail_read" => {
            let account = args["account"].as_str().unwrap_or("");
            let uid = args["uid"].as_u64().unwrap_or(0);
            if account.is_empty() || uid == 0 {
                return Err("account and uid are wanted".into());
            }
            let folder = args["folder"].as_str().unwrap_or("");
            get(port, &format!("/mail/letter?account={}&uid={}&folder={}", urlencode(account), uid, urlencode(folder)))?
        }
        "wakuwaku_mail_send" => {
            if args["account"].as_str().unwrap_or("").is_empty() || args["to"].as_str().unwrap_or("").is_empty() {
                return Err("account and to are wanted".into());
            }
            post(port, "/mail/send", args.clone())?
        }
        "wakuwaku_devservers" => get(port, "/devservers")?,
        "wakuwaku_notify" => notify(port, &args)?,
        "wakuwaku_devserver_start" => {
            let it = args["port"].as_u64().unwrap_or(0);
            if it < 1024 || it > 65535 {
                return Err("port: 1024 to 65535".into());
            }
            post(port, "/devservers/start", json!({ "port": it }))?
        }
        "wakuwaku_calendar" => {
            let date = args["date"].as_str().unwrap_or("");
            get(port, &format!("/calendar?date={}", urlencode(date)))?
        }
        "wakuwaku_calendar_add" => {
            if args["title"].as_str().unwrap_or("").trim().is_empty() {
                return Err("title is wanted".into());
            }
            post(port, "/calendar/add", args.clone())?
        }
        "wakuwaku_calendar_update" => {
            if args["id"].is_null() {
                return Err("id is wanted".into());
            }
            post(port, "/calendar/update", args.clone())?
        }
        "wakuwaku_calendar_delete" => {
            if args["id"].is_null() {
                return Err("id is wanted".into());
            }
            post(port, "/calendar/delete", args.clone())?
        }
        other => return Err(format!("unknown tool {other}")),
    };
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&got).unwrap_or_default() }],
    }))
}

fn notify(port: u16, args: &Value) -> Result<Value, String> {
    let label = args["label"].as_str().unwrap_or("").trim();
    let value = args["value"].as_str().unwrap_or("").trim();
    if label.is_empty() {
        return Err("label is wanted".into());
    }
    let mut widget = json!({
        "id": format!("mcp-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)),
        "icon": "star",
        "color": "#5e9bff",
        "label": label.chars().take(40).collect::<String>(),
        "value": value.chars().take(24).collect::<String>(),
        "ttl": 600,
        // It may open the island for this once (never while a session waits).
        "nudge": true,
    });
    if let Some(text) = args["text"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
        widget["detail"] = json!({ "extra": text.chars().take(24).collect::<String>() });
    }
    post(port, "/widget", widget)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).proxy(None).build().new_agent()
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn get(port: u16, path: &str) -> Result<Value, String> {
    match agent().get(&format!("http://127.0.0.1:{port}{path}")).call() {
        Ok(mut res) => res.body_mut().read_json().map_err(|e| e.to_string()),
        Err(e) => Err(format!("Wakuwaku is not answering on 127.0.0.1:{port} ({e}); open her and try again")),
    }
}

fn post(port: u16, path: &str, body: Value) -> Result<Value, String> {
    match agent().post(&format!("http://127.0.0.1:{port}{path}")).send_json(&body) {
        Ok(mut res) => res.body_mut().read_json().map_err(|e| e.to_string()),
        Err(e) => Err(format!("Wakuwaku is not answering on 127.0.0.1:{port} ({e}); open her and try again")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_are_whole() {
        let list = tools();
        let names: Vec<&str> = list.as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
        assert_eq!(names, [
            "wakuwaku_sessions",
            "wakuwaku_notify",
            "wakuwaku_mail",
            "wakuwaku_mail_read",
            "wakuwaku_mail_send",
            "wakuwaku_devservers",
            "wakuwaku_devserver_start",
            "wakuwaku_calendar",
            "wakuwaku_calendar_add",
            "wakuwaku_calendar_update",
            "wakuwaku_calendar_delete",
            "wakuwaku_health",
        ]);
        for tool in list.as_array().unwrap() {
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
        // The port is the one thing an agent may not leave out of starting one.
        let start = &list[6];
        assert_eq!(start["inputSchema"]["required"], json!(["port"]));
        // An event needs a title; the rest it can leave to today.
        assert_eq!(list[8]["inputSchema"]["required"], json!(["title"]));
        // Changing or removing one goes by its id.
        assert_eq!(list[9]["inputSchema"]["required"], json!(["id"]));
        assert_eq!(list[10]["inputSchema"]["required"], json!(["id"]));
    }
}
