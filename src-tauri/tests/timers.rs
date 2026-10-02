use dynamic_island_lib::providers::timers::*;

fn act(s: &mut TimersState, json: &str, now: i64) {
    apply(s, serde_json::from_str(json).unwrap(), now);
}

#[test]
fn cronometro_acumula_entre_pausas() {
    let mut s = TimersState::default();
    act(&mut s, r#"{"action":"stopwatchToggle"}"#, 1_000);
    act(&mut s, r#"{"action":"stopwatchToggle"}"#, 4_000);
    assert_eq!(s.stopwatch.accumulated_ms, 3_000);
    act(&mut s, r#"{"action":"stopwatchToggle"}"#, 10_000);
    assert_eq!(s.stopwatch.started_at_ms, Some(10_000));
}

#[test]
fn cuenta_atras_vence_y_avisa() {
    let mut s = TimersState::default();
    act(&mut s, r#"{"action":"countdownSet","minutes":1}"#, 0);
    act(&mut s, r#"{"action":"countdownToggle"}"#, 0);
    assert!(check_deadlines(&mut s, 59_999).is_empty());
    let d = check_deadlines(&mut s, 60_000);
    assert_eq!(d.len(), 1);
    assert!(!s.countdown.running);
}

#[test]
fn pomodoro_alterna_fases_y_descanso_largo() {
    let mut s = TimersState::default();
    act(&mut s, r#"{"action":"pomodoroConfig","work":1,"short":1,"long":2}"#, 0);
    act(&mut s, r#"{"action":"pomodoroToggle"}"#, 0);
    let mut now = 0;
    let mut phases = vec![];
    for _ in 0..8 {
        now = s.pomodoro.ends_at_ms.unwrap();
        check_deadlines(&mut s, now);
        phases.push(s.pomodoro.phase);
    }
    assert_eq!(phases[0], Phase::ShortBreak);
    assert_eq!(phases[1], Phase::Work);
    assert_eq!(phases[6], Phase::LongBreak);
    assert_eq!(phases[7], Phase::Work);
    assert_eq!(s.pomodoro.completed, 4);
    assert!(s.pomodoro.running && s.pomodoro.ends_at_ms.unwrap() > now);
}
