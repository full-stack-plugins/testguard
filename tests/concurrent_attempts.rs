mod common;
use testguard::{
    report::transport::FailureKind,
    runner::scheduler::{Admission, Publication, Scheduler},
};
fn request(
    plan: &testguard::plan::FrozenPlan,
    id: &str,
) -> testguard::report::transport::FixtureInvocation {
    let mut i = common::invocation(plan);
    i.run_id = id.into();
    i
}
fn record(
    plan: &testguard::plan::FrozenPlan,
    id: &str,
    fail: bool,
) -> testguard::report::AttemptRecord {
    let mut a = common::attempt(plan);
    a.attempt_id = id.into();
    let bytes = b"fixture-only execution observations";
    let r = testguard::report::normalize::ArtifactRef::from_bytes(id, "native", bytes).unwrap();
    a.artifacts = vec![r.clone()];
    for o in &mut a.observations {
        o.artifact_uri = r.uri.clone();
        if fail {
            o.status = testguard::report::CaseStatus::Fail;
        }
    }
    if fail {
        a.exit_code = Some(1);
    }
    a
}
#[test]
fn duplicate_admission_starts_once_and_late_success_cannot_replace_failure() {
    let scheduler = Scheduler::new();
    let old = common::engine_plan();
    let inv = request(&old, "old");
    let lease = match scheduler
        .admit("request-old", &old, inv.clone(), &[], &[])
        .unwrap()
    {
        Admission::Start(l) => l,
        _ => panic!("first starts"),
    };
    assert!(matches!(
        scheduler.admit("request-old", &old, inv, &[], &[]).unwrap(),
        Admission::Existing(_)
    ));
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    let childgate = gate.clone();
    let child = std::thread::spawn(move || {
        childgate.wait();
        lease.complete(&record(&old, "old", false)).unwrap()
    });
    let base = common::engine_plan();
    let mut binding = base.binding().clone();
    binding.candidate = "3".repeat(40);
    let new = testguard::plan::FrozenPlan::freeze(base.obligations(), binding).unwrap();
    let inv = request(&new, "new");
    let lease = match scheduler
        .admit("request-new", &new, inv.clone(), &[], &[])
        .unwrap()
    {
        Admission::Start(l) => l,
        _ => panic!("new starts"),
    };
    let failed = lease.complete(&record(&new, "new", true)).unwrap();
    assert_eq!(scheduler.publish(&failed).unwrap(), Publication::Current);
    gate.wait();
    let late = child.join().unwrap();
    assert_eq!(late.bundle().exit_code(), 0);
    assert_eq!(scheduler.publish(&late).unwrap(), Publication::Archived);
    assert_eq!(
        scheduler.current(&inv.binding).unwrap().unwrap().exit_code,
        Some(2)
    );
    assert_eq!(scheduler.publish(&failed).unwrap(), Publication::Duplicate);
    assert_eq!(scheduler.history().unwrap().len(), 2);
}
#[test]
fn cancellation_and_foreign_completion_never_become_current_success() {
    let s = Scheduler::new();
    let other = Scheduler::new();
    let p = common::engine_plan();
    let inv = request(&p, "cancelled");
    let lease = match s.admit("r", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let done = lease
        .fail(FailureKind::Cancelled, b"cancelled fixture")
        .unwrap();
    assert_eq!(done.bundle().exit_code(), 4);
    assert!(other.publish(&done).is_err());
    assert_eq!(s.publish(&done).unwrap(), Publication::Current);
    assert_eq!(s.current(&inv.binding).unwrap().unwrap().exit_code, Some(4));
}

#[test]
fn unpublished_completion_keeps_inflight_slot_and_dropped_or_oversized_work_is_abandoned() {
    use testguard::runner::scheduler::{Limits, Status};
    let p = common::engine_plan();
    let s = Scheduler::with_limits(Limits {
        requests: 4,
        inflight: 1,
        ..Limits::default()
    })
    .unwrap();
    let inv = request(&p, "one");
    let l = match s.admit("one", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let done = l.complete(&record(&p, "one", false)).unwrap();
    assert!(s.admit("two", &p, request(&p, "two"), &[], &[]).is_err());
    drop(done);
    assert_eq!(
        s.current(&inv.binding).unwrap().unwrap().status,
        Status::Abandoned
    );
    let l = match s.admit("two", &p, request(&p, "two"), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    drop(l);
    let small = Scheduler::with_limits(Limits {
        bundle_bytes: 1,
        ..Limits::default()
    })
    .unwrap();
    let inv = request(&p, "oversized");
    let l = match small.admit("oversized", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    assert!(l.complete(&record(&p, "oversized", false)).is_err());
    assert_eq!(
        small.current(&inv.binding).unwrap().unwrap().status,
        Status::Abandoned
    );
}

#[test]
fn simultaneous_duplicate_requests_start_one_worker_and_freeze_mapping_inputs() {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    let s = Scheduler::new();
    let p = Arc::new(common::engine_plan());
    let gate = Arc::new(Barrier::new(8));
    let starts = Arc::new(AtomicUsize::new(0));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let s = s.clone();
            let p = p.clone();
            let gate = gate.clone();
            let starts = starts.clone();
            std::thread::spawn(move || {
                gate.wait();
                match s
                    .admit("same-request", &p, request(&p, "same-run"), &[], &[])
                    .unwrap()
                {
                    Admission::Start(l) => {
                        starts.fetch_add(1, Ordering::SeqCst);
                        let done = l.complete(&record(&p, "same-run", false)).unwrap();
                        s.publish(&done).unwrap();
                    }
                    Admission::Existing(_) => (),
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(s.history().unwrap().len(), 1);
    assert!(
        s.admit(
            "same-request",
            &p,
            request(&p, "same-run"),
            &[testguard::policy::Weakening::TestRemoved],
            &[]
        )
        .is_err()
    );
    assert!(
        s.admit(
            "same-request",
            &p,
            request(&p, "same-run"),
            &[],
            &["changed advice".into()]
        )
        .is_err()
    );
    assert!(
        s.admit("new-request", &p, request(&p, "same-run"), &[], &[])
            .is_err()
    );
    let inv = request(&p, "review-run");
    let lease = match s
        .admit(
            "review",
            &p,
            inv.clone(),
            &[testguard::policy::Weakening::TestRemoved],
            &[],
        )
        .unwrap()
    {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    assert_eq!(s.current(&inv.binding).unwrap().unwrap().exit_code, None);
    let done = lease.complete(&record(&p, "review-run", false)).unwrap();
    assert_eq!(done.bundle().exit_code(), 3);
    s.publish(&done).unwrap();
    let history = s.history().unwrap();
    let last = history.last().unwrap();
    assert_eq!(
        &*last.raw_bundle,
        serde_json::to_vec(done.bundle()).unwrap()
    );
}

#[test]
fn exact_current_binding_and_separate_scopes_never_cross_satisfy() {
    let s = Scheduler::new();
    let p = common::engine_plan();
    let inv = request(&p, "one");
    let l = match s.admit("one", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let done = l.complete(&record(&p, "one", false)).unwrap();
    s.publish(&done).unwrap();
    for case in 0..5 {
        let mut b = inv.binding.clone();
        match case {
            0 => b.candidate_oid = "a".repeat(40),
            1 => b.base_oid = "b".repeat(40),
            2 => b.source_snapshot_digest = format!("sha256:{}", "c".repeat(64)),
            3 => b.merge_group_id = Some("other".into()),
            _ => b.baseline_digest = None,
        };
        assert!(s.current(&b).unwrap().is_none());
    }
    let mut independent = request(&p, "independent");
    independent.binding.task_id = "other-task".into();
    let l = match s
        .admit("independent", &p, independent.clone(), &[], &[])
        .unwrap()
    {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let failed = l.fail(FailureKind::Runtime, b"fixture failure").unwrap();
    s.publish(&failed).unwrap();
    assert_eq!(s.current(&inv.binding).unwrap().unwrap().exit_code, Some(0));
    assert_eq!(
        s.current(&independent.binding).unwrap().unwrap().exit_code,
        Some(4)
    );
}

#[test]
fn count_byte_identity_and_history_limits_fail_closed_without_restoring_old_green() {
    use testguard::runner::scheduler::{Limits, Status};
    let p = common::engine_plan();
    let s = Scheduler::with_limits(Limits {
        requests: 2,
        inflight: 1,
        history_bytes: 1,
        ..Limits::default()
    })
    .unwrap();
    assert!(
        s.admit(&"x".repeat(257), &p, request(&p, "one"), &[], &[])
            .is_err()
    );
    let mut huge = request(&p, "huge");
    huge.binding.task_id = "x".repeat(65 * 1024);
    assert!(s.admit("huge", &p, huge, &[], &[]).is_err());
    assert!(
        s.admit(
            "advice",
            &p,
            request(&p, "advice"),
            &[],
            &vec!["a".into(); 65]
        )
        .is_err()
    );
    let inv = request(&p, "one");
    let l = match s.admit("one", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let done = l.complete(&record(&p, "one", false)).unwrap();
    assert!(s.publish(&done).is_err());
    assert_eq!(s.current(&inv.binding).unwrap().unwrap().exit_code, None);
    assert!(s.history().unwrap().is_empty());
    drop(done);
    let l = match s.admit("two", &p, request(&p, "two"), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    assert!(
        l.fail(FailureKind::Runtime, &vec![0; 256 * 1024 + 1])
            .is_err()
    );
    assert_eq!(
        s.current(&inv.binding).unwrap().unwrap().status,
        Status::Abandoned
    );
    assert!(
        s.admit("three", &p, request(&p, "three"), &[], &[])
            .is_err()
    );
}

#[test]
fn requirement_scopes_are_independent_and_invalid_worker_record_stays_bound_error() {
    let s = Scheduler::new();
    let a = common::engine_plan();
    let mut obligations = common::obligations();
    obligations["requirements"] = serde_json::json!(["OTHER-1", "OTHER-2"]);
    obligations["sources"][0]["requirement_id"] = "OTHER-1".into();
    obligations["sources"][1]["requirement_id"] = "OTHER-2".into();
    let b = testguard::plan::FrozenPlan::freeze(
        &testguard::obligation::ObligationSet::parse(&obligations.to_string()).unwrap(),
        a.binding().clone(),
    )
    .unwrap();
    let ia = request(&a, "requirements-a");
    let ib = request(&b, "requirements-b");
    let la = match s.admit("a", &a, ia.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let lb = match s.admit("b", &b, ib.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let good = la.complete(&record(&a, "requirements-a", false)).unwrap();
    s.publish(&good).unwrap();
    let invalid = lb.complete(&record(&a, "foreign-attempt", false)).unwrap();
    assert_eq!(invalid.bundle().exit_code(), 4);
    assert_eq!(invalid.bundle().envelope.binding, ib.binding);
    s.publish(&invalid).unwrap();
    assert_eq!(s.current(&ia.binding).unwrap().unwrap().exit_code, Some(0));
    assert_eq!(s.current(&ib.binding).unwrap().unwrap().exit_code, Some(4));
}
