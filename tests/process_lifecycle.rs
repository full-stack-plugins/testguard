#![cfg(target_os = "linux")]
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    time::Instant,
};
use testguard::runner::process::{Config, Fixture, Outcome, ProcessReport};
fn run(fixture: Fixture, cancel: Option<bool>, limit: usize) -> ProcessReport {
    run_control(fixture, cancel.map(|eof| if eof { 2 } else { 1 }), limit)
}
fn run_control(fixture: Fixture, cancel: Option<u8>, limit: usize) -> ProcessReport {
    let directory = tempfile::tempdir().unwrap();
    let config = Config {
        fixture,
        timeout_ms: 350,
        cleanup_ms: 500,
        output_bytes: limit,
    };
    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_testguard-local-lifecycle"))
        .arg("--local-fixture-v1")
        .arg(serde_json::to_string(&config).unwrap())
        .current_dir(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut control = child.stdin.take().unwrap();
    if let Some(mode) = cancel {
        let mut ready = String::new();
        BufReader::new(child.stderr.as_mut().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready, "READY\n");
        if mode >= 3 {
            rustix::process::kill_process(
                rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
                if mode == 3 {
                    rustix::process::Signal::TERM
                } else {
                    rustix::process::Signal::INT
                },
            )
            .unwrap();
        } else if mode == 2 {
            drop(control);
        } else {
            control.write_all(b"cancel\ncancel\n").unwrap();
            drop(control);
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(started.elapsed().as_secs() < 3);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    serde_json::from_slice(&output.stdout).unwrap()
}
fn assert_tree_gone(report: &ProcessReport) {
    assert!(report.cleanup_complete, "{report:?}");
    let text = String::from_utf8_lossy(&report.stdout);
    let pids: Vec<_> = text
        .lines()
        .filter_map(|l| l.strip_prefix("PID "))
        .collect();
    assert_eq!(pids.len(), 2, "{text}");
    for pid in pids {
        assert!(
            !std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "surviving or unreaped process {pid}"
        );
    }
    assert!(text.contains("<failure>fixture failure</failure>"));
}
#[test]
fn actual_children_and_grandchildren_are_reaped_on_timeout() {
    let r = run(Fixture::Tree, None, 4096);
    assert_eq!(r.outcome, Outcome::TimedOut);
    assert_eq!(r.exit_code, None);
    assert_tree_gone(&r);
}
#[test]
fn repeated_cancel_and_control_eof_cleanup_real_tree() {
    for eof in [false, true] {
        let r = run(Fixture::Tree, Some(eof), 4096);
        assert_eq!(r.outcome, Outcome::Cancelled);
        assert_tree_gone(&r);
    }
}
#[test]
fn both_output_pipes_are_bounded_and_overflow_never_completes() {
    let r = run(Fixture::Flood, None, 1024);
    assert_eq!(r.outcome, Outcome::OutputLimit);
    assert!(r.cleanup_complete);
    assert!(r.stdout.len() <= 1024);
    assert!(r.stderr.len() <= 1024);
    assert!(!r.stdout.is_empty());
    assert!(!r.stderr.is_empty());
}
#[test]
fn native_exit_failure_is_preserved_and_completion_is_not_test_success() {
    for (fixture, code) in [(Fixture::Pass, 0), (Fixture::Fail, 1)] {
        let r = run(fixture, None, 4096);
        assert_eq!(r.outcome, Outcome::Completed);
        assert_eq!(r.exit_code, Some(code));
        assert!(r.cleanup_complete);
    }
}

#[test]
fn signal_interrupts_clean_up_the_real_tree() {
    for mode in [3, 4] {
        let r = run_control(Fixture::Tree, Some(mode), 4096);
        assert_eq!(r.outcome, Outcome::Cancelled);
        assert_tree_gone(&r);
    }
}

#[test]
fn denied_cleanup_and_exceeded_cleanup_budget_never_report_complete() {
    for fixture in [Fixture::CleanupDenied, Fixture::CleanupStalled] {
        let r = run(fixture, None, 4096);
        assert_eq!(r.outcome, Outcome::CleanupFailed);
        assert!(!r.cleanup_complete);
        assert_eq!(r.exit_code, None);
        // The test fault is followed by best-effort real cleanup, without
        // laundering the first failed cleanup into a complete report.
        let text = String::from_utf8_lossy(&r.stdout);
        assert!(text.contains("<failure>fixture failure</failure>"));
        for pid in text.lines().filter_map(|l| l.strip_prefix("PID ")) {
            assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        }
    }
}

#[test]
fn rejected_profiles_and_budgets_do_not_start_a_command_or_emit_complete() {
    for input in [
        r#"{"fixture":"shell","timeout_ms":350,"cleanup_ms":500,"output_bytes":4096}"#,
        r#"{"fixture":"tree","timeout_ms":0,"cleanup_ms":500,"output_bytes":4096}"#,
        r#"{"fixture":"tree","timeout_ms":350,"cleanup_ms":0,"output_bytes":4096}"#,
        r#"{"fixture":"tree","timeout_ms":350,"cleanup_ms":500,"output_bytes":999999999}"#,
        r#"{"fixture":"pass","timeout_ms":350,"cleanup_ms":500,"output_bytes":4096,"command":"sh"}"#,
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_testguard-local-lifecycle"))
            .args(["--local-fixture-v1", input])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
