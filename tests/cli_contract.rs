mod common;
use std::process::Command;
#[test]
fn check_exit_codes_and_stream_separation() {
    let dir = std::env::temp_dir().join(format!("testguard-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plan = common::plan();
    let full = common::attempt(&plan);
    let p = dir.join("plan.json");
    let a = dir.join("attempt.json");
    let w = dir.join("changes.json");
    std::fs::write(&p, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(&w, b"[\"filter_changed\"]").unwrap();
    for expected in [0, 2, 3] {
        let mut attempt = full.clone();
        if expected == 2 {
            attempt.observations.pop();
        }
        std::fs::write(&a, serde_json::to_vec(&attempt).unwrap()).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_testguard"));
        cmd.arg("check").arg(&p).arg(&a);
        if expected == 3 {
            cmd.arg(&w);
        }
        let output = cmd.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["profile"], "local-fixture-advisory");
        assert!(output.stderr.is_empty());
    }
    let error = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .args(["check", "missing"])
        .output()
        .unwrap();
    assert_eq!(error.status.code(), Some(4));
    assert!(error.stdout.is_empty());
    assert!(!error.stderr.is_empty());
}
#[test]
fn doctor_and_run_are_separated() {
    let doctor = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("doctor")
        .output()
        .unwrap();
    assert!(doctor.status.success());
    assert!(serde_json::from_slice::<serde_json::Value>(&doctor.stdout).is_ok());
    let run = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("run")
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(4));
    assert!(run.stdout.is_empty());
}
#[test]
fn verify_recomputes_block_and_rejects_report_tampering() {
    let dir = std::env::temp_dir().join(format!("testguard-verify-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plan = common::plan();
    let mut attempt = common::attempt(&plan);
    attempt.observations.pop();
    let p = dir.join("plan");
    let a = dir.join("attempt");
    let r = dir.join("report");
    std::fs::write(&p, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(&a, serde_json::to_vec(&attempt).unwrap()).unwrap();
    let report = testguard::coverage::assess(&plan, &attempt, &[]).unwrap();
    std::fs::write(&r, serde_json::to_vec(&report).unwrap()).unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("verify")
        .arg(&p)
        .arg(&a)
        .arg(&r)
        .output()
        .unwrap();
    assert_eq!(verified.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&verified.stderr).contains("authenticity not verified"));
    let mut bad = serde_json::to_value(report).unwrap();
    bad["decision"] = "ALLOW".into();
    std::fs::write(&r, serde_json::to_vec(&bad).unwrap()).unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("verify")
        .arg(&p)
        .arg(&a)
        .arg(&r)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(4));
    assert!(rejected.stdout.is_empty());
}
