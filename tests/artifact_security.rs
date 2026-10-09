use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, cargo, junit},
    report::{CaseStatus, normalize::ArtifactRef},
};

fn profile(tool: &str) -> ExecutorProfile {
    ExecutorProfile {
        tool: tool.into(),
        version: if tool == "cargo" { "1.99.0" } else { "3.5.2" }.into(),
        protocol: if tool == "cargo" {
            "libtest-pretty-v1"
        } else {
            "junit-xml-v1"
        }
        .into(),
        target: "lib".into(),
        features: vec![],
        parameters: String::new(),
        environment: "linux".into(),
    }
}
fn raw(output: &str, inventory: &str, exit: i32) -> RawArtifactSet {
    RawArtifactSet {
        attempt_id: "security".into(),
        inventory: inventory.into(),
        output: output.into(),
        artifact: ArtifactRef::from_bytes("security", "report", output.as_bytes()).unwrap(),
        exit_code: Some(exit),
        interrupted: false,
    }
}
#[test]
fn cargo_rejects_oversized_report_before_treating_native_zero_as_evidence() {
    let output = format!(
        "{}\nrunning 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        "x".repeat(4 * 1024 * 1024)
    );
    assert!(cargo::parse(&raw(&output, "", 0), &profile("cargo")).is_err());
}
#[test]
fn profile_scope_cannot_amplify_into_every_observation() {
    let inventory = (0..100)
        .map(|n| format!("case{n}: test\n"))
        .collect::<String>();
    let output = format!(
        "running 100 tests\n{}test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        (0..100)
            .map(|n| format!("test case{n} ... ok\n"))
            .collect::<String>()
    );
    let mut p = profile("cargo");
    p.parameters = "x".repeat(128 * 1024);
    assert!(cargo::parse(&raw(&output, &inventory, 0), &p).is_err());
}
#[test]
fn junit_bounds_nodes_and_rejects_entities_depth_and_truncation() {
    let xml = format!(
        "<testsuite name=\"s\" tests=\"0\" failures=\"0\" errors=\"0\" skipped=\"0\"><properties>{}</properties></testsuite>",
        "<property name=\"x\" value=\"y\"/>".repeat(20_000)
    );
    assert!(junit::parse(&raw(&xml, "", 0), &profile("maven-surefire")).is_err());
    for xml in [
        "<!DOCTYPE s [<!ENTITY steal SYSTEM 'file:///etc/passwd'>]><testsuite/>".to_string(),
        format!("{}{}", "<x>".repeat(512), "</x>".repeat(512)),
        "<testsuite name=\"truncated\"".to_string(),
    ] {
        assert!(junit::parse(&raw(&xml, "", 0), &profile("maven-surefire")).is_err());
    }
    let failed = include_str!("../fixtures/junit/captured/fail/report.input.xml");
    assert_eq!(
        junit::parse(&raw(failed, "", 1), &profile("maven-surefire")).unwrap()[0].status,
        CaseStatus::Fail
    );
}

#[cfg(target_os = "linux")]
#[test]
fn collector_confines_reads_and_keeps_failed_report_when_another_artifact_is_bad() {
    use std::{fs, os::unix::fs::symlink, path::Path};
    use testguard::runner::collect::{ArtifactCollector, Request};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let failed = include_bytes!("../fixtures/junit/captured/fail/report.input.xml");
    fs::create_dir(root.path().join("reports")).unwrap();
    fs::write(root.path().join("reports/fail.xml"), failed).unwrap();
    let reference = ArtifactRef::from_bytes("security", "report", failed).unwrap();
    let collector = ArtifactCollector::open(root.path()).unwrap();
    assert_eq!(
        collector
            .read(Path::new("reports/fail.xml"), &reference, "security")
            .unwrap()
            .bytes,
        failed
    );
    let mut changed = failed.to_vec();
    changed[0] ^= 1;
    fs::write(root.path().join("tampered.xml"), changed).unwrap();
    let mut tampered_reference = reference.clone();
    tampered_reference.uri = "evidence://security/tampered".into();
    let batch = collector.collect_batch(
        &[
            Request {
                path: "reports/fail.xml".into(),
                reference: reference.clone(),
            },
            Request {
                path: "tampered.xml".into(),
                reference: tampered_reference,
            },
        ],
        "security",
    );
    assert!(!batch.is_complete());
    assert_eq!(batch.errors.len(), 1);
    assert!(batch.errors[0].contains("digest conflict"));
    assert_eq!(batch.artifacts.len(), 1);
    let text = std::str::from_utf8(&batch.artifacts[0].bytes).unwrap();
    assert_eq!(
        junit::parse(&raw(text, "", 1), &profile("maven-surefire")).unwrap()[0].status,
        CaseStatus::Fail
    );
    fs::write(outside.path().join("secret"), failed).unwrap();
    symlink(outside.path(), root.path().join("alias")).unwrap();
    symlink(outside.path().join("secret"), root.path().join("link.xml")).unwrap();
    fs::hard_link(outside.path().join("secret"), root.path().join("hard.xml")).unwrap();
    for path in [
        "../secret",
        "/etc/passwd",
        "alias/secret",
        "link.xml",
        "hard.xml",
        "reports/./fail.xml",
    ] {
        assert!(
            collector
                .read(Path::new(path), &reference, "security")
                .is_err(),
            "{path}"
        );
    }
    for (name, bytes) in [
        ("gzip", b"\x1f\x8b\x08\x00".as_slice()),
        ("zip", b"PK\x03\x04".as_slice()),
    ] {
        fs::write(root.path().join(name), bytes).unwrap();
        let reference = ArtifactRef::from_bytes("security", name, bytes).unwrap();
        assert!(
            collector
                .read(Path::new(name), &reference, "security")
                .is_err()
        );
    }
    let compressed = include_bytes!("../fixtures/security/oversized-report.gz");
    fs::write(root.path().join("compressed-report"), compressed).unwrap();
    let compressed_reference = ArtifactRef::from_bytes("security", "gzip", compressed).unwrap();
    assert!(
        collector
            .read(
                Path::new("compressed-report"),
                &compressed_reference,
                "security"
            )
            .err()
            .unwrap()
            .contains("no decompression")
    );
    let file = fs::File::create(root.path().join("huge")).unwrap();
    file.set_len(4 * 1024 * 1024 + 1).unwrap();
    let huge_reference =
        ArtifactRef::from_bytes("security", "huge", &vec![0; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(
        collector
            .read(Path::new("huge"), &huge_reference, "security")
            .is_err()
    );
    assert_eq!(fs::read(outside.path().join("secret")).unwrap(), failed);
}

#[cfg(target_os = "linux")]
#[test]
fn collection_byte_boundary_and_request_count_are_explicit() {
    use std::{fs, path::Path};
    use testguard::runner::collect::{ArtifactCollector, Request};
    let root = tempfile::tempdir().unwrap();
    let bytes = vec![b'x'; 4 * 1024 * 1024];
    let reference = ArtifactRef::from_bytes("security", "boundary", &bytes).unwrap();
    fs::write(root.path().join("boundary"), &bytes).unwrap();
    let collector = ArtifactCollector::open(root.path()).unwrap();
    assert_eq!(
        collector
            .read(Path::new("boundary"), &reference, "security")
            .unwrap()
            .bytes,
        bytes
    );
    let requests: Vec<_> = (0..65)
        .map(|n| Request {
            path: format!("missing{n}").into(),
            reference: reference.clone(),
        })
        .collect();
    let batch = collector.collect_batch(&requests, "security");
    assert!(!batch.is_complete());
    assert_eq!(batch.errors, ["collection request count limit"]);
    assert!(batch.artifacts.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn collector_pins_root_and_rejects_fifo_without_blocking() {
    use std::{
        fs,
        os::unix::fs::symlink,
        path::Path,
        time::{Duration, Instant},
    };
    use testguard::runner::collect::ArtifactCollector;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("report"), b"original").unwrap();
    let collector = ArtifactCollector::open(&root).unwrap();
    let reference = ArtifactRef::from_bytes("security", "report", b"original").unwrap();
    fs::rename(&root, parent.path().join("old-root")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("report"), b"replacement").unwrap();
    assert_eq!(
        collector
            .read(Path::new("report"), &reference, "security")
            .unwrap()
            .bytes,
        b"original"
    );
    let alias = parent.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(ArtifactCollector::open(&alias).is_err());
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        parent.path().join("old-root/fifo"),
        rustix::fs::Mode::RUSR,
    )
    .unwrap();
    let start = Instant::now();
    assert!(
        collector
            .read(Path::new("fifo"), &reference, "security")
            .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(fs::read(root.join("report")).unwrap(), b"replacement");
}

#[test]
fn xml_preflight_preserves_literal_markup_and_bounds_deep_input_before_recursion() {
    let xml = r#"<testsuite name="s" tests="1" failures="0" errors="0" skipped="0"><!-- <!DOCTYPE is just comment text --><testcase classname="C" name="x"><system-out><![CDATA[<not-an-element>]]></system-out></testcase></testsuite>"#;
    assert_eq!(
        junit::parse(&raw(xml, "", 0), &profile("maven-surefire")).unwrap()[0].status,
        CaseStatus::Pass
    );
    let deep = format!("{}{}", "<x>".repeat(10_000), "</x>".repeat(10_000));
    let error = junit::parse(&raw(&deep, "", 0), &profile("maven-surefire")).unwrap_err();
    assert!(error.contains("nesting limit"));
}
