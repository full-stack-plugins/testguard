mod common;
use std::process::Command;
#[test]
fn engine_cli_separates_prebinding_errors_and_bound_parse_errors_and_recomputes_reports() {
    let dir = std::env::temp_dir().join(format!("testguard-engine-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plan = common::engine_plan();
    let plan_path = dir.join("plan.json");
    let invocation = dir.join("invocation.json");
    let attempt = dir.join("attempt.json");
    let bundle_path = dir.join("bundle.json");
    std::fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(
        &invocation,
        serde_json::to_vec(&common::invocation(&plan)).unwrap(),
    )
    .unwrap();
    let changes = dir.join("changes.json");
    std::fs::write(&changes, b"[\"assertion_removed\"]").unwrap();
    for code in [0, 2, 3] {
        let mut a = common::attempt(&plan);
        if code == 2 {
            a.observations.pop();
        }
        std::fs::write(&attempt, serde_json::to_vec(&a).unwrap()).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_testguard"));
        cmd.arg("check-engine")
            .arg(&plan_path)
            .arg(&invocation)
            .arg(&attempt);
        if code == 3 {
            cmd.arg(&changes);
        }
        let output = cmd.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let bundle: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(bundle["capability"], "testguard.engine-fixture/v1");
        assert_eq!(bundle["envelope"]["runStatus"], "completed");
        std::fs::write(&bundle_path, &output.stdout).unwrap();
        let verify = Command::new(env!("CARGO_BIN_EXE_testguard"))
            .arg("verify-engine")
            .arg(&plan_path)
            .arg(&bundle_path)
            .output()
            .unwrap();
        assert_eq!(verify.status.code(), Some(code));
        assert!(String::from_utf8_lossy(&verify.stderr).contains("not authenticate"));
    }
    std::fs::write(&attempt, b"{malformed").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("check-engine")
        .arg(&plan_path)
        .arg(&invocation)
        .arg(&attempt)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    let bundle: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(bundle["envelope"]["decision"].is_null());
    assert_eq!(bundle["envelope"]["runStatus"], "error");
    std::fs::write(&invocation, b"{}").unwrap();
    let pre = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("check-engine")
        .arg(&plan_path)
        .arg(&invocation)
        .arg(&attempt)
        .output()
        .unwrap();
    assert_eq!(pre.status.code(), Some(4));
    assert!(pre.stdout.is_empty());
    assert!(!pre.stderr.is_empty());
}
