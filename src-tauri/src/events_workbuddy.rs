// WorkBuddy hook events → pet messages: Claude Code's (events.rs), plus agent:
// "workbuddy". WorkBuddy (Tencent's AI office agent) reads Claude Code's hooks
// from ~/.workbuddy-ai/settings.json (or the legacy ~/.workbuddy/settings.json)
// and runs connection::WORKBUDDY_FLAG, which hands each event to
// /hook?agent=workbuddy (main.rs workbuddy_hook). Its payloads are Claude
// Code's, tools and all. State and notification only: its approvals stay in
// its own sandbox and dialog.
use serde_json::{json, Value};

pub fn to_message(e: &Value) -> Option<Value> {
    let mut message = crate::events::to_message(e)?;
    message["agent"] = json!("workbuddy");
    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbuddy_events_become_messages() {
        let e = json!({
            "cwd": "Z:\\nowhere\\wb", "hook_event_name": "UserPromptSubmit", "session_id": "sess_w1",
            "prompt": "Write the report", "transcript_path": "",
        });
        let m = to_message(&e).unwrap();
        assert_eq!((m["agent"].as_str(), m["event"].as_str(), m["task"].as_str()), (Some("workbuddy"), Some("turn-start"), Some("Write the report")));
        let tool = json!({
            "cwd": "Z:\\nowhere\\wb", "hook_event_name": "PreToolUse", "session_id": "sess_w1",
            "tool_name": "Bash", "tool_input": { "command": "npm test", "description": "Run the tests" }, "tool_use_id": "t1",
            "transcript_path": "",
        });
        let m = to_message(&tool).unwrap();
        assert_eq!((m["detail"].as_str(), m["agent"].as_str()), (Some("Bash"), Some("workbuddy")));
    }
}
