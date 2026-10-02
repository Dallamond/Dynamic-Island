use dynamic_island_lib::providers::agent::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn ciclo_de_un_turno() {
    let mut m = HashMap::new();
    let base = json!({"session_id": "s1", "cwd": r"D:\Proyectos\isla"});
    let ev = |e: &str, extra: serde_json::Value| {
        let mut v = base.clone();
        v["hook_event_name"] = json!(e);
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    };
    assert!(apply_hook(&mut m, &ev("UserPromptSubmit", json!({"prompt_text": "hola"}))));
    assert_eq!(m["s1"].status, Status::Working);
    assert_eq!(m["s1"].project, "isla");
    apply_hook(&mut m, &ev("PreToolUse", json!({"tool_name": "Edit", "tool_input": {"file_path": "C:/x/src/lib.rs"}})));
    assert_eq!(m["s1"].last_action.as_deref(), Some("Edit lib.rs"));
    assert_eq!(m["s1"].files, vec!["lib.rs"]);
    apply_hook(&mut m, &ev("Notification", json!({"notification_type": "permission_prompt", "message": "Bash quiere ejecutar"})));
    assert_eq!(m["s1"].status, Status::Waiting);
    apply_hook(&mut m, &ev("PostToolUse", json!({"tool_name": "Bash"})));
    assert_eq!(m["s1"].status, Status::Working);
    apply_hook(&mut m, &ev("Stop", json!({"last_assistant_message": "Hecho"})));
    assert_eq!(m["s1"].status, Status::Done);
    assert!(m["s1"].turn_ended_ms.is_some());
    assert!(apply_hook(&mut m, &ev("SessionEnd", json!({}))));
    assert!(m.is_empty());
}

#[test]
fn statusline_rellena_uso() {
    let mut m = HashMap::new();
    apply_status(&mut m, &json!({
        "session_id": "s1", "cwd": "/p",
        "model": {"display_name": "Opus"},
        "cost": {"total_cost_usd": 1.5},
        "context_window": {"used_percentage": 42, "context_window_size": 200000, "total_input_tokens": 84000},
        "rate_limits": {"five_hour": {"used_percentage": 23.5, "resets_at": 1738425600}}
    }));
    let u = &m["s1"].usage;
    assert_eq!(u.model.as_deref(), Some("Opus"));
    assert_eq!(u.context_pct, Some(42.0));
    assert_eq!(u.five_hour_pct, Some(23.5));
    assert_eq!(u.seven_day_pct, None);
}
