//! Read-only local CLI contracts. The execution command deliberately stays disabled.
use crate::{
    obligation::ObligationSet,
    plan::{Binding, FrozenPlan},
    policy::Weakening,
    report::AttemptRecord,
};
use serde::Serialize;
use std::io::{Read, Write};
const MAX_INPUT: usize = 1024 * 1024;
const MAX_OUTPUT: usize = 32 * 1024 * 1024;
const MAX_ARG_BYTES: usize = 4096;
const MAX_ARG_TOTAL: usize = 16384;
#[derive(Clone, Copy, Debug)]
enum Error {
    Arguments,
    ArgvLimit,
    ArgvEncoding,
    EnvironmentLimit,
    Unavailable,
    InputLimit,
    Structure,
    Invalid,
    Verification,
    ExecutionDisabled,
    ExecutionFailed,
    OutputLimit,
    OutputUnavailable,
}
impl Error {
    fn code(self) -> &'static str {
        match self {
            Self::Arguments => "argv.invalid",
            Self::ArgvLimit => "argv.limit",
            Self::ArgvEncoding => "argv.encoding",
            Self::EnvironmentLimit => "environment.limit",
            Self::Unavailable => "input.unavailable",
            Self::InputLimit => "input.limit",
            Self::Structure => "input.structure",
            Self::Invalid => "input.invalid",
            Self::Verification => "verification.failed",
            Self::ExecutionDisabled => "execution.disabled",
            Self::ExecutionFailed => "execution.failed",
            Self::OutputLimit => "output.limit",
            Self::OutputUnavailable => "output.unavailable",
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Arguments => "invalid command or exact argument count",
            Self::ArgvLimit => "command argument budget exceeded",
            Self::ArgvEncoding => "command arguments must be UTF-8",
            Self::EnvironmentLimit => "doctor PATH budget exceeded",
            Self::Unavailable => "input must be an accessible regular file without a final symlink",
            Self::InputLimit => "input byte budget exceeded",
            Self::Structure => "input JSON structural budget exceeded",
            Self::Invalid => "input schema or frozen binding is invalid",
            Self::Verification => "report differs from required recomputation",
            Self::ExecutionDisabled => {
                "execution unavailable: reviewed sandbox and explicit execution permission required"
            }
            Self::ExecutionFailed => "bound fixture processing failed without a result",
            Self::OutputLimit => "result exceeds serialization budget; no JSON result emitted",
            Self::OutputUnavailable => "result output unavailable",
        }
    }
    fn emit(self) {
        let kind = if matches!(
            self,
            Self::OutputLimit | Self::OutputUnavailable | Self::ExecutionFailed
        ) {
            "CliDiagnostic"
        } else {
            "PreBindingDiagnostic"
        };
        let value = serde_json::json!({"schema_version":"testguard.cli-diagnostic/v1","kind":kind,"code":self.code(),"message":self.message()});
        let _ = writeln!(std::io::stderr().lock(), "{value}");
    }
}
struct Output {
    bytes: Vec<u8>,
    exit: i32,
}
struct BoundedOutput {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("output limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn output<T: Serialize>(value: &T, exit: i32) -> Result<Output, Error> {
    let mut writer = BoundedOutput {
        bytes: Vec::new(),
        limit: MAX_OUTPUT,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| Error::OutputLimit)?;
    Ok(Output {
        bytes: writer.bytes,
        exit,
    })
}
fn notice(engine: bool) {
    let message = if engine {
        "engine and domain recomputation consistent; does not authenticate producer, approval or Git objects"
    } else {
        "consistent local report; producer and approval authenticity not verified"
    };
    let value = serde_json::json!({"schema_version":"testguard.cli-diagnostic/v1","kind":"ConsistencyNotice","message":message});
    let _ = writeln!(std::io::stderr().lock(), "{value}");
}
fn read_bytes(path: &str, limit: usize) -> Result<Vec<u8>, Error> {
    #[cfg(target_os = "linux")]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| Error::Unavailable)?
    };
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (path, limit);
        return Err(Error::Unavailable);
    }
    #[cfg(target_os = "linux")]
    {
        let meta = file.metadata().map_err(|_| Error::Unavailable)?;
        if !meta.is_file() {
            return Err(Error::Unavailable);
        }
        if meta.len() > limit as u64 {
            return Err(Error::InputLimit);
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Unavailable)?;
        if bytes.len() > limit {
            return Err(Error::InputLimit);
        }
        Ok(bytes)
    }
}
fn json_preflight(bytes: &[u8]) -> Result<(), Error> {
    let (mut quoted, mut escaped, mut depth, mut tokens) = (false, false, 0usize, 0usize);
    for &b in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
            continue;
        }
        match b {
            b'"' => {
                quoted = true;
                tokens += 1;
            }
            b'{' | b'[' => {
                depth += 1;
                tokens += 1;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).ok_or(Error::Structure)?;
            }
            b',' => tokens += 1,
            _ => {}
        }
        if depth > 64 || tokens > 4194304 {
            return Err(Error::Structure);
        }
    }
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    json_preflight(bytes)?;
    serde_json::from_slice(bytes).map_err(|_| Error::Invalid)
}
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Error> {
    decode(&read_bytes(path, MAX_INPUT)?)
}
fn plan(path: &str) -> Result<FrozenPlan, Error> {
    let bytes = read_bytes(path, MAX_INPUT)?;
    json_preflight(&bytes)?;
    FrozenPlan::parse(std::str::from_utf8(&bytes).map_err(|_| Error::Invalid)?)
        .map_err(|_| Error::Invalid)
}
fn command(args: &[String]) -> Result<Output, Error> {
    let cmd = args.first().map(String::as_str).ok_or(Error::Arguments)?;
    // Named engine modes and existing spellings share exactly one implementation.
    if matches!(cmd, "check" | "verify") && args.get(1).is_some_and(|a| a == "--engine") {
        let mut translated = vec![format!("{cmd}-engine")];
        translated.extend_from_slice(&args[2..]);
        return command(&translated);
    }
    match cmd {
        "check-engine" if (4..=6).contains(&args.len()) => {
            use crate::report::transport::{FailureKind, FixtureInvocation, prepare};
            let plan = plan(&args[1])?;
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
            let bound = prepare(&plan, invocation).map_err(|_| Error::Invalid)?;
            let result=match read_bytes(&args[3],MAX_INPUT){
    Err(error)=>bound.fail(FailureKind::Runtime,match error {Error::InputLimit=>b"attempt rejected by CLI byte budget; original input not retained",_=>b"attempt input unavailable as a regular file; original input not retained"}),
    Ok(bytes)=>match decode::<AttemptRecord>(&bytes){Err(_)=>bound.fail(FailureKind::Parser,&bytes),Ok(attempt)=>bound.complete(&attempt,&changes,&advice)},
   }.map_err(|_|Error::ExecutionFailed)?;
            output(&result, result.exit_code())
        }
        "verify-engine" if args.len() == 3 => {
            let plan = plan(&args[1])?;
            let bundle: crate::report::envelope::FixtureBundle =
                decode(&read_bytes(&args[2], MAX_OUTPUT)?)?;
            crate::report::envelope::verify_bundle(&plan, &bundle)
                .map_err(|_| Error::Verification)?;
            let encoded = output(&bundle.envelope, bundle.exit_code())?;
            notice(true);
            Ok(encoded)
        }
        "doctor" if args.len() == 1 => {
            let path = std::env::var_os("PATH").unwrap_or_default();
            if path.as_encoded_bytes().len() > MAX_ARG_TOTAL
                || std::env::split_paths(&path).take(129).count() > 128
            {
                return Err(Error::EnvironmentLimit);
            }
            output(&crate::doctor::discover(&path), 0)
        }
        "plan" if args.len() == 3 => {
            let bytes = read_bytes(&args[1], MAX_INPUT)?;
            json_preflight(&bytes)?;
            let source =
                ObligationSet::parse(std::str::from_utf8(&bytes).map_err(|_| Error::Invalid)?)
                    .map_err(|_| Error::Invalid)?;
            let binding: Binding = read(&args[2])?;
            output(
                &FrozenPlan::freeze(&source, binding).map_err(|_| Error::Invalid)?,
                0,
            )
        }
        "check" | "verify" | "coverage" => {
            let valid = match cmd {
                "verify" => args.len() == 4 || args.len() == 5,
                "coverage" => args.len() == 3,
                _ => args.len() == 3 || args.len() == 4,
            };
            if !valid {
                return Err(Error::Arguments);
            }
            let plan = plan(&args[1])?;
            let attempt: AttemptRecord = read(&args[2])?;
            let changes: Vec<Weakening> =
                if let Some(path) = args.get(if cmd == "verify" { 4 } else { 3 }) {
                    read(path)?
                } else {
                    vec![]
                };
            let assessment =
                crate::coverage::assess(&plan, &attempt, &changes).map_err(|_| Error::Invalid)?;
            if cmd == "coverage" {
                return output(&assessment.coverage, 0);
            }
            let encoded = output(&assessment, assessment.decision.exit_code())?;
            if cmd == "verify" {
                let expected: serde_json::Value = read(&args[3])?;
                let actual: serde_json::Value = decode(&encoded.bytes)?;
                if expected != actual {
                    return Err(Error::Verification);
                }
            }
            if cmd == "verify" {
                notice(false);
            }
            Ok(encoded)
        }
        "run" => Err(Error::ExecutionDisabled),
        _ => Err(Error::Arguments),
    }
}
fn valid_args(args: &[String]) -> Result<(), Error> {
    if args.len() > 8
        || args.iter().any(|a| a.len() > MAX_ARG_BYTES)
        || args.iter().fold(0usize, |n, a| n.saturating_add(a.len())) > MAX_ARG_TOTAL
    {
        Err(Error::ArgvLimit)
    } else {
        Ok(())
    }
}
pub fn run(args: Vec<String>) -> i32 {
    match valid_args(&args).and_then(|()| command(&args)) {
        Ok(result) => {
            let mut stdout = std::io::stdout().lock();
            if stdout
                .write_all(&result.bytes)
                .and_then(|()| stdout.write_all(b"\n"))
                .is_err()
            {
                Error::OutputUnavailable.emit();
                4
            } else {
                result.exit
            }
        }
        Err(error) => {
            error.emit();
            4
        }
    }
}
/// Inspect argument bytes before converting or collecting an unbounded command vector.
pub fn run_os(args: impl IntoIterator<Item = std::ffi::OsString>) -> i32 {
    let mut frozen = Vec::new();
    let mut total = 0usize;
    for arg in args {
        total = total.saturating_add(arg.as_encoded_bytes().len());
        if frozen.len() >= 8
            || arg.as_encoded_bytes().len() > MAX_ARG_BYTES
            || total > MAX_ARG_TOTAL
        {
            Error::ArgvLimit.emit();
            return 4;
        }
        match arg.into_string() {
            Ok(arg) => frozen.push(arg),
            Err(_) => {
                Error::ArgvEncoding.emit();
                return 4;
            }
        }
    }
    run(frozen)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_limit_returns_no_publishable_result() {
        let payload = "x".repeat(MAX_OUTPUT);
        assert!(matches!(output(&payload, 0), Err(Error::OutputLimit)));
        let ordinary = output(&serde_json::json!({"ok":true}), 0).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&ordinary.bytes).unwrap(),
            serde_json::json!({"ok":true})
        );
    }
}
