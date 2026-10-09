//! Fixed Linux lifecycle fixture profile. This is not an execution sandbox.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fixture {
    Pass,
    Fail,
    Tree,
    Flood,
    CleanupDenied,
    CleanupStalled,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub fixture: Fixture,
    pub timeout_ms: u64,
    pub cleanup_ms: u64,
    pub output_bytes: usize,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Completed,
    TimedOut,
    Cancelled,
    OutputLimit,
    CleanupFailed,
    Error,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessReport {
    pub profile: String,
    pub outcome: Outcome,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub cleanup_complete: bool,
}
pub const PROFILE: &str = "testguard.lifecycle/linux-fixture-v1";
/// Entry point for the dedicated helper only. Never call inside a host application:
/// subreaper and waitpid ownership belong to this standalone process.
pub fn helper_main() -> i32 {
    #[cfg(target_os = "linux")]
    {
        linux::main()
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("unsupported: Linux local lifecycle fixture only");
        4
    }
}
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use rustix::process::{self, Pid, Signal, WaitOptions};
    use std::{
        io::{Read, Write},
        os::unix::process::CommandExt,
        process::{Command, Stdio},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };
    static INTERRUPTED: AtomicBool = AtomicBool::new(false);
    extern "C" fn interrupted(_: libc::c_int) {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }
    #[derive(Default)]
    struct Capture {
        bytes: Vec<u8>,
        overflow: bool,
        error: bool,
        done: bool,
    }
    fn capture(mut input: impl Read + Send + 'static, limit: usize) -> Arc<Mutex<Capture>> {
        let state = Arc::new(Mutex::new(Capture::default()));
        let writer = state.clone();
        thread::spawn(move || {
            let mut chunk = [0; 4096];
            loop {
                match input.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut s = writer.lock().unwrap();
                        let remaining = limit - s.bytes.len();
                        let keep = remaining.min(n);
                        s.bytes.extend_from_slice(&chunk[..keep]);
                        s.overflow |= n > remaining;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        writer.lock().unwrap().error = true;
                        break;
                    }
                }
            }
            writer.lock().unwrap().done = true;
        });
        state
    }
    fn reap(main: Pid, exit: &mut Option<i32>) -> Result<bool, String> {
        loop {
            match process::wait(WaitOptions::NOHANG) {
                Ok(Some((pid, status))) => {
                    if pid == main {
                        *exit = status.exit_status();
                    }
                }
                Ok(None) => return Ok(false),
                Err(rustix::io::Errno::CHILD) => return Ok(true),
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    fn cleanup(
        main: Pid,
        exit: &mut Option<i32>,
        budget: Duration,
        mut kill: impl FnMut(Pid) -> rustix::io::Result<()>,
    ) -> bool {
        let end = Instant::now() + budget;
        loop {
            // ESRCH is normal when the original group has already exited.
            if let Err(e) = kill(main)
                && e != rustix::io::Errno::SRCH
            {
                return false;
            }
            match reap(main, exit) {
                Ok(true) => return Instant::now() <= end,
                Err(_) => return false,
                Ok(false) => {}
            }
            if Instant::now() >= end {
                return false;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    fn supervise(config: Config) -> Result<ProcessReport, String> {
        if !(10..=10_000).contains(&config.timeout_ms)
            || !(10..=2_000).contains(&config.cleanup_ms)
            || !(128..=65_536).contains(&config.output_bytes)
        {
            return Err("invalid local fixture resource budget".into());
        }
        // This standalone helper owns its signal disposition. The handler only
        // stores into a lock-free atomic; it performs no allocation or I/O.
        unsafe {
            if libc::signal(
                libc::SIGTERM,
                interrupted as *const () as libc::sighandler_t,
            ) == libc::SIG_ERR
                || libc::signal(libc::SIGINT, interrupted as *const () as libc::sighandler_t)
                    == libc::SIG_ERR
            {
                return Err("cannot install interruption handler".into());
            }
        }
        process::set_child_subreaper(Some(process::getpid())).map_err(|e| e.to_string())?;
        let fixture = match config.fixture {
            Fixture::Pass => "pass",
            Fixture::Fail => "fail",
            Fixture::Tree | Fixture::CleanupDenied | Fixture::CleanupStalled => "tree",
            Fixture::Flood => "flood",
        };
        // No caller-provided executable, argv, environment, cwd or shell is accepted.
        let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .args(["--internal-fixture", fixture])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|e| e.to_string())?;
        let pid = Pid::from_raw(child.id() as i32).ok_or("invalid child pid")?;
        let out = capture(
            child.stdout.take().ok_or("missing stdout")?,
            config.output_bytes,
        );
        let err = capture(
            child.stderr.take().ok_or("missing stderr")?,
            config.output_bytes,
        );
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        thread::spawn(move || {
            let mut byte = [0];
            loop {
                match std::io::stdin().read(&mut byte) {
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => {
                        flag.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });
        let start = Instant::now();
        let mut exit = None;
        let mut ready = false;
        let mut outcome = loop {
            {
                let o = out.lock().unwrap();
                let e = err.lock().unwrap();
                if !ready && o.bytes.windows(6).any(|w| w == b"READY\n") {
                    eprintln!("READY");
                    ready = true;
                }
                if o.overflow || e.overflow {
                    break Outcome::OutputLimit;
                }
                if o.error || e.error {
                    break Outcome::Error;
                }
            }
            if cancelled.load(Ordering::SeqCst) || INTERRUPTED.load(Ordering::SeqCst) {
                break Outcome::Cancelled;
            }
            match process::waitpid(Some(pid), WaitOptions::NOHANG) {
                Ok(Some((_, status))) => {
                    exit = status.exit_status();
                    break if exit.is_some() {
                        Outcome::Completed
                    } else {
                        Outcome::Error
                    };
                }
                Ok(None) => {}
                Err(rustix::io::Errno::INTR) => continue,
                Err(_) => break Outcome::Error,
            }
            if start.elapsed() >= Duration::from_millis(config.timeout_ms) {
                break Outcome::TimedOut;
            }
            thread::sleep(Duration::from_millis(2));
        };
        let cleanup_started = Instant::now();
        let budget = Duration::from_millis(config.cleanup_ms);
        let injected_fault = matches!(
            config.fixture,
            Fixture::CleanupDenied | Fixture::CleanupStalled
        );
        let mut clean = cleanup(pid, &mut exit, budget, |pid| match config.fixture {
            Fixture::CleanupDenied => Err(rustix::io::Errno::PERM),
            Fixture::CleanupStalled => Ok(()), // injected ineffective kill: real tree stays alive
            _ => process::kill_process_group(pid, Signal::KILL),
        });
        if !clean && injected_fault {
            // Test-only fault profiles get an additional bounded rescue. Preserve
            // CleanupFailed even if this rescue succeeds; never claim budget success.
            let _ = cleanup(pid, &mut exit, Duration::from_millis(100), |pid| {
                process::kill_process_group(pid, Signal::KILL)
            });
        }
        while clean && !(out.lock().unwrap().done && err.lock().unwrap().done) {
            if cleanup_started.elapsed() >= budget {
                clean = false;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let o = out.lock().unwrap();
        let e = err.lock().unwrap();
        if !clean {
            outcome = Outcome::CleanupFailed;
        } else if o.overflow || e.overflow {
            outcome = Outcome::OutputLimit;
        } else if o.error || e.error {
            outcome = Outcome::Error;
        }
        if outcome != Outcome::Completed {
            exit = None;
        }
        Ok(ProcessReport {
            profile: PROFILE.into(),
            outcome,
            exit_code: exit,
            stdout: o.bytes.clone(),
            stderr: e.bytes.clone(),
            cleanup_complete: clean,
        })
    }
    fn fixture(mode: &str) -> i32 {
        match mode {
            "pass" => {
                println!("fixture passed");
                0
            }
            "fail" => {
                println!("<failure>fixture failure</failure>");
                1
            }
            "leaf" => loop {
                thread::sleep(Duration::from_secs(60));
            },
            "tree" => {
                let mut child = Command::new(std::env::current_exe().unwrap())
                    .args(["--internal-fixture", "leaf"])
                    .stdin(Stdio::null())
                    .spawn()
                    .unwrap();
                println!(
                    "PID {}\nPID {}\n<failure>fixture failure</failure>\nREADY",
                    std::process::id(),
                    child.id()
                );
                std::io::stdout().flush().unwrap();
                let _ = child.wait();
                1
            }
            "flood" => {
                let bytes = [b'x'; 512];
                loop {
                    if std::io::stdout().write_all(&bytes).is_err()
                        || std::io::stderr().write_all(&bytes).is_err()
                    {
                        return 1;
                    }
                }
            }
            _ => 4,
        }
    }
    pub fn main() -> i32 {
        let args: Vec<_> = std::env::args().take(4).collect();
        if args.len() == 3 && args[1] == "--internal-fixture" {
            return fixture(&args[2]);
        }
        if args.len() != 3 || args[1] != "--local-fixture-v1" || args[2].len() > 2048 {
            eprintln!("only the fixed local fixture protocol is supported");
            return 4;
        }
        let result = serde_json::from_str(&args[2])
            .map_err(|e| e.to_string())
            .and_then(supervise);
        match result {
            Ok(report) => {
                println!("{}", serde_json::to_string(&report).unwrap());
                if report.outcome == Outcome::Completed {
                    0
                } else {
                    4
                }
            }
            Err(e) => {
                eprintln!("{e}");
                4
            }
        }
    }
}
