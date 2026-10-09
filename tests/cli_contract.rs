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

#[test]
fn named_engine_mode_runs_real_engine_and_preserves_old_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let plan = common::engine_plan();
    let p = dir.path().join("plan");
    let i = dir.path().join("invocation");
    let a = dir.path().join("attempt");
    let w = dir.path().join("changes");
    let b = dir.path().join("bundle");
    std::fs::write(&p, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(&i, serde_json::to_vec(&common::invocation(&plan)).unwrap()).unwrap();
    std::fs::write(&w, b"[\"filter_changed\"]").unwrap();
    for code in [0, 2, 3, 4] {
        let mut attempt = common::attempt(&plan);
        if code == 2 {
            attempt.observations.pop();
        }
        std::fs::write(
            &a,
            if code == 4 {
                b"{malformed".to_vec()
            } else {
                serde_json::to_vec(&attempt).unwrap()
            },
        )
        .unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_testguard"));
        cmd.args(["check", "--engine"]).arg(&p).arg(&i).arg(&a);
        if code == 3 {
            cmd.arg(&w);
        }
        let out = cmd.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stderr.is_empty());
        let bundle: testguard::report::envelope::FixtureBundle =
            serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(bundle.capability, "testguard.engine-fixture/v1");
        if code == 4 {
            assert!(bundle.envelope.decision.is_none());
            continue;
        }
        testguard::report::envelope::verify_bundle(&plan, &bundle).unwrap();
        std::fs::write(&b, &out.stdout).unwrap();
        let verified = Command::new(env!("CARGO_BIN_EXE_testguard"))
            .args(["verify", "--engine"])
            .arg(&p)
            .arg(&b)
            .output()
            .unwrap();
        assert_eq!(verified.status.code(), Some(code));
        let notice: serde_json::Value = serde_json::from_slice(&verified.stderr).unwrap();
        assert_eq!(notice["kind"], "ConsistencyNotice");
    }
}

#[test]
fn every_read_only_command_has_a_machine_contract_and_run_stays_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let plan = common::plan();
    let source = dir.path().join("source");
    let binding = dir.path().join("binding");
    let p = dir.path().join("plan");
    let a = dir.path().join("attempt");
    std::fs::write(&source, serde_json::to_vec(plan.obligations()).unwrap()).unwrap();
    std::fs::write(&binding, serde_json::to_vec(plan.binding()).unwrap()).unwrap();
    let planned = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("plan")
        .arg(&source)
        .arg(&binding)
        .output()
        .unwrap();
    assert_eq!(planned.status.code(), Some(0));
    assert!(planned.stderr.is_empty());
    assert_eq!(
        serde_json::to_value(
            testguard::plan::FrozenPlan::parse(std::str::from_utf8(&planned.stdout).unwrap())
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&plan).unwrap()
    );
    std::fs::write(&p, &planned.stdout).unwrap();
    let mut partial = common::attempt(&plan);
    partial.observations.pop();
    std::fs::write(&a, serde_json::to_vec(&partial).unwrap()).unwrap();
    let coverage = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("coverage")
        .arg(&p)
        .arg(&a)
        .output()
        .unwrap();
    assert_eq!(coverage.status.code(), Some(0));
    assert!(coverage.stderr.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&coverage.stdout).unwrap();
    assert!(
        v["execution"]["numerator"].as_u64().unwrap()
            < v["execution"]["denominator"].as_u64().unwrap()
    );
    for args in [
        vec!["run", "/bin/sh", "-c", "echo should-not-run"],
        vec!["unknown", "private-canary-input"],
        vec!["check", "missing-private-canary-path"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        let diagnostic: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(diagnostic["kind"], "PreBindingDiagnostic");
        assert!(diagnostic.get("binding").is_none());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("canary"));
    }
}

#[test]
fn input_and_argv_budgets_fail_before_binding_and_after_binding_stays_null() {
    let dir = tempfile::tempdir().unwrap();
    let plan = common::engine_plan();
    let p = dir.path().join("plan");
    let i = dir.path().join("invocation");
    let a = dir.path().join("attempt");
    std::fs::write(&p, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(&i, serde_json::to_vec(&common::invocation(&plan)).unwrap()).unwrap();
    std::fs::write(&a, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("check-engine")
        .arg(&p)
        .arg(&i)
        .arg(&a)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stderr.is_empty());
    let bundle: testguard::report::envelope::FixtureBundle =
        serde_json::from_slice(&out.stdout).unwrap();
    assert!(bundle.envelope.decision.is_none());
    assert!(
        bundle
            .artifacts
            .iter()
            .filter(|(uri, _)| uri.ends_with("failed-input.bin"))
            .all(|(_, bytes)| bytes.len() < 256)
    );
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .args(["plan", a.to_str().unwrap(), p.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "input.limit");
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("doctor")
        .arg("x".repeat(4097))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "argv.limit");
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_and_non_regular_inputs_are_static_rejections() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg(std::ffi::OsString::from_vec(vec![255]))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "argv.encoding");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let link = dir.path().join("link");
    std::fs::write(&source, b"{}").unwrap();
    symlink(&source, &link).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("plan")
        .arg(&link)
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "input.unavailable");
}

#[cfg(target_os = "linux")]
#[test]
fn fifo_deep_json_and_oversized_doctor_path_are_bounded_rejections() {
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let fifo = dir.path().join("fifo");
    let other = dir.path().join("other");
    std::fs::write(&other, b"{}").unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("plan")
        .arg(&fifo)
        .arg(&other)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("CLI blocked opening FIFO");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "input.unavailable");
    std::fs::write(&other, format!("{}0{}", "[".repeat(65), "]".repeat(65))).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("plan")
        .arg(&other)
        .arg(&other)
        .output()
        .unwrap();
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "input.structure");
    let out = Command::new(env!("CARGO_BIN_EXE_testguard"))
        .arg("doctor")
        .env("PATH", "x".repeat(16385))
        .output()
        .unwrap();
    assert!(out.stdout.is_empty());
    let d: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(d["code"], "environment.limit");
}
