use crate::{
    obligation::ObligationSet,
    plan::{Binding, FrozenPlan},
    policy::Weakening,
    report::AttemptRecord,
};
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn command(args: &[String]) -> Result<(serde_json::Value, i32), String> {
    let cmd = args
        .first()
        .map(String::as_str)
        .ok_or("expected doctor/plan/check/verify/coverage/run")?;
    match cmd {
        "check-engine" if (4..=6).contains(&args.len()) => {
            use crate::report::transport::{FailureKind, FixtureInvocation, prepare};
            let plan =
                FrozenPlan::parse(&std::fs::read_to_string(&args[1]).map_err(|e| e.to_string())?)?;
            let invocation: FixtureInvocation = read(&args[2])?;
            let changes: Vec<Weakening> = if let Some(path) = args.get(4) {
                read(path)?
            } else {
                vec![]
            };
            let advice: Vec<String> = if let Some(path) = args.get(5) {
                read(path)?
            } else {
                vec![]
            };
            let bound =
                prepare(&plan, invocation).map_err(|e| format!("{}: {}", e.code, e.message))?;
            let result = match std::fs::read(&args[3]) {
                Err(_) => bound.fail(FailureKind::Runtime, b"attempt input unavailable"),
                Ok(bytes) => match serde_json::from_slice::<AttemptRecord>(&bytes) {
                    Err(_) => bound.fail(FailureKind::Parser, &bytes),
                    Ok(attempt) => bound.complete(&attempt, &changes, &advice),
                },
            }
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
            let exit = result.exit_code();
            Ok((
                serde_json::to_value(result).map_err(|e| e.to_string())?,
                exit,
            ))
        }
        "verify-engine" if args.len() == 3 => {
            let plan =
                FrozenPlan::parse(&std::fs::read_to_string(&args[1]).map_err(|e| e.to_string())?)?;
            let bundle: crate::report::envelope::FixtureBundle = read(&args[2])?;
            crate::report::envelope::verify_bundle(&plan, &bundle)?;
            eprintln!(
                "engine and domain recomputation consistent; does not authenticate producer, approval or Git objects"
            );
            let exit = bundle.exit_code();
            Ok((
                serde_json::to_value(bundle.envelope).map_err(|e| e.to_string())?,
                exit,
            ))
        }
        "doctor" if args.len() == 1 => Ok((
            serde_json::to_value(crate::doctor::discover(
                &std::env::var_os("PATH").unwrap_or_default(),
            ))
            .map_err(|e| e.to_string())?,
            0,
        )),
        "plan" if args.len() == 3 => {
            let source = ObligationSet::parse(
                &std::fs::read_to_string(&args[1]).map_err(|e| e.to_string())?,
            )?;
            let binding: Binding = read(&args[2])?;
            Ok((
                serde_json::to_value(FrozenPlan::freeze(&source, binding)?)
                    .map_err(|e| e.to_string())?,
                0,
            ))
        }
        "check" | "coverage" | "verify" => {
            let valid = match cmd {
                "verify" => args.len() == 4 || args.len() == 5,
                "coverage" => args.len() == 3,
                _ => args.len() == 3 || args.len() == 4,
            };
            if !valid {
                return Err("expected command PLAN ATTEMPT [CHANGES], or verify PLAN ATTEMPT REPORT [CHANGES]".into());
            }
            let plan =
                FrozenPlan::parse(&std::fs::read_to_string(&args[1]).map_err(|e| e.to_string())?)?;
            let attempt: AttemptRecord = read(&args[2])?;
            let change_index = if cmd == "verify" { 4 } else { 3 };
            let changes: Vec<Weakening> = if let Some(path) = args.get(change_index) {
                read(path)?
            } else {
                vec![]
            };
            let assessment = crate::coverage::assess(&plan, &attempt, &changes)?;
            let exit = assessment.decision.exit_code();
            let output = serde_json::to_value(&assessment).map_err(|e| e.to_string())?;
            if cmd == "verify" {
                let expected: serde_json::Value = read(&args[3])?;
                if expected != output {
                    return Err("report differs from recomputation".into());
                }
                eprintln!(
                    "consistent local report; producer and approval authenticity not verified"
                );
            }
            if cmd == "coverage" {
                Ok((
                    serde_json::to_value(assessment.coverage).map_err(|e| e.to_string())?,
                    0,
                ))
            } else {
                Ok((output, exit))
            }
        }
        "run" => Err(
            "execution unavailable: reviewed sandbox and explicit execution permission required"
                .into(),
        ),
        _ => Err("invalid command or arguments".into()),
    }
}
pub fn run(args: Vec<String>) -> i32 {
    match command(&args) {
        Ok((value, code)) => {
            println!(
                "{}",
                serde_json::to_string(&value).expect("JSON value serializes")
            );
            code
        }
        Err(error) => {
            eprintln!("testguard: {error}");
            4
        }
    }
}
