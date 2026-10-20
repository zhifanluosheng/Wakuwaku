// Trae CN hook events → pet messages: Claude Code's (events.rs), plus agent:
// "trae". Trae runs its hooks from ~/.trae-cn/hooks.json (Claude Code's shape:
// { matcher, hooks: [{ type, command }] }) as this program with
// connection::TRAE_FLAG (an encoded command, its Windows sandbox splits quoted
// paths), which hands each to /hook?agent=trae (main.rs trae_hook). Its fields
// are Claude Code's (session_id, hook_event_name, tool_name, tool_input,
// tool_use_id, prompt, cwd, notification_type); its tools are its own names
// (RunCommand, ReadFile, WriteFile, EditFile...), renamed to Claude Code's
// here. State only: Trae's permission prompts stay in Trae.
use serde_json::{json, Value};

fn claude_tool(name: &str) -> &str {
    match name {
        "RunCommand" | "run_command" | "runCommand" | "Bash" => "Bash",
        "ReadFile" | "read_file" | "Read" => "Read",
        "WriteFile" | "write_file" | "Write" => "Write",
        "EditFile" | "edit_file" | "ApplyPatch" | "apply_patch" | "Edit" => "Edit",
        "Search" | "search" | "Grep" | "GrepSearch" => "Grep",
        "Glob" | "glob" | "Find" | "FileSearch" => "Glob",
        "WebSearch" | "web_search" | "SearchWeb" => "WebSearch",
        "WebFetch" | "web_fetch" | "Fetch" => "WebFetch",
        "Task" | "Agent" | "task" => "Task",
        "AskUserQuestion" | "ask_user_question" | "Ask" => "AskUserQuestion",
        "TodoWrite" | "todo_write" | "Todo" => "TodoWrite",
        other => other,
    }
}

pub fn to_message(e: &Value) -> Option<Value> {
    let mut e = e.clone();
    if let Some(o) = e.as_object_mut() {
        if let Some(name) = o.get("tool_name").and_then(Value::as_str) {
            o.insert("tool_name".into(), json!(claude_tool(name)));
        }
        // Its own names for a file: path, filePath.
        if let Some(Value::Object(input)) = o.get_mut("tool_input") {
            if !input.contains_key("file_path") {
                if let Some(path) = input.get("path").or_else(|| input.get("filePath")).cloned() {
                    input.insert("file_path".into(), path);
                }
            }
        }
    }
    let mut message = crate::events::to_message(&e)?;
    message["agent"] = json!("trae");
    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(name: &str, more: Value) -> Value {
        let mut e = json!({
            "cwd": "Z:\\nowhere\\trae", "hook_event_name": name, "session_id": "sess_t1",
            "transcript_path": "", "permission_mode": "default",
        });
        for (k, v) in more.as_object().unwrap() {
            e[k] = v.clone();
        }
        e
    }

    #[test]
    fn trae_events_become_messages() {
        let m = to_message(&event("SessionStart", json!({ "source": "startup" }))).unwrap();
        assert_eq!((m["mood"].as_str(), m["agent"].as_str(), m["session"].as_str()), (Some("idle"), Some("trae"), Some("sess_t1")));
        let m = to_message(&event("UserPromptSubmit", json!({ "prompt": "Fix the build" }))).unwrap();
        assert_eq!((m["event"].as_str(), m["task"].as_str()), (Some("turn-start"), Some("Fix the build")));
        let m = to_message(&event("PreToolUse", json!({ "tool_name": "RunCommand", "tool_use_id": "call_1", "tool_input": { "command": "npm test", "description": "Run the tests" } }))).unwrap();
        assert_eq!((m["detail"].as_str(), m["step"]["key"].as_str()), (Some("Bash"), Some("step.run")));
        let m = to_message(&event("PostToolUse", json!({ "tool_name": "WriteFile", "tool_use_id": "call_2", "tool_input": { "path": "Z:\\nowhere\\trae\\note.txt", "content": "hi" } }))).unwrap();
        assert_eq!((m["event"].as_str(), m["file"].as_str()), (Some("tool-done"), Some("Z:\\nowhere\\trae\\note.txt")));
        let m = to_message(&event("Stop", json!({ "last_assistant_message": "done" }))).unwrap();
        assert_eq!((m["mood"].as_str(), m["reply"].as_str()), (Some("done"), Some("done")));
    }

    #[test]
    fn trae_tools_are_claude_codes() {
        for (theirs, ours) in [
            ("RunCommand", "Bash"),
            ("ReadFile", "Read"),
            ("WriteFile", "Write"),
            ("EditFile", "Edit"),
            ("Search", "Grep"),
            ("Find", "Glob"),
            ("Ask", "AskUserQuestion"),
        ] {
            assert_eq!(claude_tool(theirs), ours);
        }
    }
}
