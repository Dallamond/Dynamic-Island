use dynamic_island_lib::providers::agent::{AgentSession, Status};
use dynamic_island_lib::providers::codex::*;
use serde_json::json;

#[test]
fn fecha_iso_a_epoch() {
    assert_eq!(parse_ts("1970-01-01T00:00:00.000Z"), Some(0));
    assert_eq!(parse_ts("2026-10-01T11:50:00.106Z"), Some(1_790_855_400_106));
    assert_eq!(parse_ts("2024-02-29T23:59:59Z"), Some(1_709_251_199_000));
    assert_eq!(parse_ts("basura"), None);
}

#[test]
fn ciclo_de_un_turno_de_codex() {
    let mut s = AgentSession { id: "c1".into(), agent: CODEX.into(), ..Default::default() };
    let ev = |t: &str, p: serde_json::Value| json!({"timestamp": "2026-10-01T11:50:00.000Z", "type": t, "payload": p});
    apply_line(&mut s, &ev("session_meta", json!({"id": "c1", "cwd": r"D:\Lukaton1"})));
    assert_eq!(s.project, "Lukaton1");
    apply_line(&mut s, &ev("turn_context", json!({"model": "gpt-6-luna", "cwd": r"D:\Proyectos\web"})));
    assert_eq!(s.usage.model.as_deref(), Some("gpt-6-luna"));
    assert_eq!(s.project, "web");
    apply_line(&mut s, &ev("event_msg", json!({"type": "task_started", "started_at": 1790855399, "model_context_window": 258400})));
    assert_eq!(s.status, Status::Working);
    assert_eq!(s.turn_started_ms, Some(1_790_855_399_000));
    apply_line(&mut s, &ev("response_item", json!({"type": "function_call", "name": "shell_command", "arguments": "{\"command\":\"npm test\"}"})));
    assert_eq!(s.last_action.as_deref(), Some("Shell npm test"));
    apply_line(&mut s, &ev("response_item", json!({"type": "custom_tool_call", "name": "apply_patch",
        "input": "*** Begin Patch\n*** Update File: src/app.ts\n@@\n*** Add File: C:/x/nuevo.css\n*** End Patch"})));
    assert_eq!(s.last_action.as_deref(), Some("Edit app.ts"));
    assert_eq!(s.files, vec!["app.ts", "nuevo.css"]);
    assert_eq!(s.tool_count, 2);
    apply_line(&mut s, &ev("event_msg", json!({"type": "token_count",
        "info": {"total_token_usage": {"output_tokens": 171}, "last_token_usage": {"total_tokens": 25840}, "model_context_window": 258400},
        "rate_limits": {"primary": {"used_percent": 8, "window_minutes": 43200, "resets_at": 1791842861}, "secondary": null}})));
    assert_eq!(s.usage.context_pct, Some(10.0));
    assert_eq!(s.usage.limits.len(), 1);
    assert_eq!(s.usage.limits[0].label, "Mes");
    assert_eq!(s.usage.limits[0].pct, Some(8.0));
    apply_line(&mut s, &ev("event_msg", json!({"type": "task_complete", "completed_at": 1790855450, "last_agent_message": "Hecho"})));
    assert_eq!(s.status, Status::Done);
    assert_eq!(s.turn_ended_ms, Some(1_790_855_450_000));
    assert_eq!(s.message.as_deref(), Some("Hecho"));
    assert!(!apply_line(&mut s, &ev("response_item", json!({"type": "reasoning"}))));
}

#[test]
fn exec_en_modo_codigo() {
    let cmd = r#"const r = await tools.exec_command({cmd:"Get-Content 'C:\\Users\\x\\SKILL.md'","workdir":"D:\\L"}); text(r.output);"#;
    assert_eq!(describe_exec(cmd), r"Shell Get-Content 'C:\Users\x\SKILL.md'");
    let web = r#"const r = await tools.web__run({search_query:[{q:"precio plus"}]}); text(r);"#;
    assert_eq!(describe_exec(web), "Buscar precio plus");
    assert_eq!(describe_exec("await tools.view_image({path:\"a.png\"})"), "view_image");
}
