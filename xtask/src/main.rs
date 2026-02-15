use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build riscv-tests and run them on your emulator
    TestRiscv(TestRiscvArgs),
}

#[derive(Parser, Debug)]
struct TestRiscvArgs {
    /// Path to riscv-tests repo (default: apps/riscv-tests)
    #[arg(long, default_value = "apps/riscv-tests")]
    riscv_tests: PathBuf,

    /// Which ISA subsets to run (repeatable)
    /// Example: --suite rv64ui-p --suite rv64mi-p --suite rv64si-p
    #[arg(long = "suite", required = true)]
    suites: Vec<String>,

    /// Path to your emulator binary
    #[arg(long)]
    emulator: PathBuf,

    /// Build riscv-tests before running
    #[arg(long, default_value_t = true)]
    build: bool,

    /// Only run tests whose filename contains this substring
    #[arg(long)]
    filter: Option<String>,

    /// Timeout per test in seconds (NOTE: not enforced here; prefer emulator's own timeout)
    #[arg(long, default_value_t = 2)]
    timeout_sec: u64,

    /// Everything after `--` is passed to emulator verbatim.
    ///
    /// Example:
    ///   xtask test-riscv --suite rv64ui-p --emulator target/release/rv-emu -- --base-addr 0x80000000
    #[arg(trailing_var_arg = true)]
    emu_args: Vec<String>,

    /// Print each emulator command line before running (debug)
    #[arg(long, default_value_t = false)]
    print_cmd: bool,

    /// Mark failure if stdout/stderr contains these substrings (repeatable).
    /// Useful when emulator returns exit code 0 but prints an error.
    #[arg(long = "fail-on-output")]
    fail_on_output: Vec<String>,

    /// Directory to write per-test logs (default: target/xtask-logs)
    #[arg(long, default_value = "target/xtask-logs")]
    log_dir: PathBuf,

    /// If set, keep logs even for passed tests (default: false)
    #[arg(long, default_value_t = false)]
    keep_pass_logs: bool,
}

#[derive(Default)]
struct SuiteSummary {
    passed: usize,
    failed: Vec<Failure>,
}

struct Failure {
    suite: String,
    test_name: String,
    test_path: PathBuf,
    code: i32, // -1 means "could not run / no exit code"
    reason: String,
    log_path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::TestRiscv(args) => test_riscv(args),
    }
}

fn test_riscv(args: TestRiscvArgs) -> Result<()> {
    let TestRiscvArgs {
        riscv_tests,
        suites,
        emulator,
        build,
        filter,
        timeout_sec: _,
        emu_args,
        print_cmd,
        fail_on_output,
        log_dir,
        keep_pass_logs,
    } = args;

    if suites.is_empty() {
        bail!("no suites specified. Use --suite rv64ui-p (repeatable).");
    }

    // 1) Ensure emulator exists
    if !emulator.exists() {
        bail!(
            "emulator not found at {:?}. Build it (e.g. cargo build --release) or pass --emulator.",
            emulator
        );
    }

    // 2) Build riscv-tests (suite targets first, fallback to `isa` once)
    if !riscv_tests.exists() {
        bail!(
            "riscv-tests repo not found at {:?}. Add it (submodule/vendor) or pass --riscv-tests.",
            riscv_tests
        );
    }

    let prefix =
        std::env::var("RISCV_PREFIX").unwrap_or_else(|_| "riscv64-unknown-elf-".to_string());

    if build {
        eprintln!(
            "[xtask] building riscv-tests suites={:?} with RISCV_PREFIX={prefix} ...",
            suites
        );

        let mut any_failed = false;
        for suite in &suites {
            if build_suite(&riscv_tests, &prefix, suite).is_err() {
                any_failed = true;
            }
        }

        if any_failed {
            eprintln!(
                "[xtask] some suite target builds failed; falling back to `make -C riscv-tests isa` ..."
            );
            run(Command::new("make")
                .arg("-C")
                .arg(&riscv_tests)
                .arg(format!("RISCV_PREFIX={prefix}"))
                .arg("isa"))?;
        }
    } else {
        eprintln!("[xtask] build skipped (--build=false)");
    }

    // 3) Discover tests per suite
    let isa_dir = riscv_tests.join("isa");
    if !isa_dir.exists() {
        bail!("expected riscv-tests build dir at {:?}. Build may have failed.", isa_dir);
    }

    // Default output markers (tuned to your example)
    let mut markers = vec![
        "Test failed".to_string(),
        "panic".to_string(),
    ];
    markers.extend(fail_on_output);

    // Prepare log dir
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log dir {:?}", log_dir))?;

    let mut summaries: BTreeMap<String, SuiteSummary> = BTreeMap::new();

    for suite in &suites {
        let tests = discover_tests(&isa_dir, suite, filter.as_deref())?;
        if tests.is_empty() {
            eprintln!("[xtask] WARN: no tests found for suite={suite} under {:?}", isa_dir);
        } else {
            eprintln!("[xtask] suite={suite}: discovered {} tests", tests.len());
        }

        let entry = summaries.entry(suite.clone()).or_default();

        // 4) Run tests (sequential, keep going)
        for t in tests {
            let test_name = t.file_name().and_then(OsStr::to_str).unwrap_or("<nonutf8>").to_string();
            eprint!("[xtask] RUN  {suite}/{test_name} ... ");

            let mut cmd = Command::new(&emulator);
            cmd.arg("--elf").arg(&t);
            for a in &emu_args {
                cmd.arg(a);
            }
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            if print_cmd {
                eprintln!("\n[xtask] cmd: {:?}", cmd);
                eprint!("[xtask] RUN  {suite}/{test_name} ... ");
            }

            // Run emulator; never `?` here; record and continue.
            let out = match cmd.output() {
                Ok(out) => out,
                Err(e) => {
                    let lp = write_log(
                        &log_dir,
                        suite,
                        &test_name,
                        &format!("spawn error: {e}\n"),
                        "",
                        &cmd,
                    )?;
                    entry.failed.push(Failure {
                        suite: suite.clone(),
                        test_name,
                        test_path: t.clone(),
                        code: -1,
                        reason: format!("failed to spawn/run emulator: {e}"),
                        log_path: lp,
                    });
                    eprintln!("FAIL (spawn error)");
                    continue;
                }
            };

            let code = out.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

            let mut is_fail = !out.status.success();

            // If emulator sometimes exits 0 but prints error, treat as failure by markers.
            if !is_fail {
                let hay = format!("{stderr}\n{stdout}");
                if markers.iter().any(|m| hay.contains(m)) {
                    is_fail = true;
                }
            }

            if !is_fail {
                entry.passed += 1;
                eprintln!("ok");

                if keep_pass_logs {
                    let _ = write_log(&log_dir, suite, &test_name, &stdout, &stderr, &cmd)?;
                }
            } else {
                let reason = if !out.status.success() {
                    format!("non-zero exit (code={code})")
                } else {
                    "matched failure marker in output".to_string()
                };

                let lp = write_log(&log_dir, suite, &test_name, &stdout, &stderr, &cmd)?;

                entry.failed.push(Failure {
                    suite: suite.clone(),
                    test_name,
                    test_path: t.clone(),
                    code,
                    reason,
                    log_path: lp,
                });

                eprintln!("FAIL");
            }
        }
    }

    // 5) Summary (suite + total)
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;

    eprintln!("\n[xtask] suite summary:");
    for (suite, s) in &summaries {
        total_passed += s.passed;
        total_failed += s.failed.len();
        eprintln!("  {suite}: passed={} failed={}", s.passed, s.failed.len());
    }

    eprintln!("\n[xtask] total: passed={total_passed} failed={total_failed}");

    if total_failed > 0 {
        eprintln!("\n[xtask] failed tests (logs written under {:?}):", log_dir);
        for (suite, s) in &summaries {
            for f in &s.failed {
                eprintln!(
                    "  {}/{} ({}, code={}) log={}",
                    suite,
                    f.test_name,
                    f.reason,
                    f.code,
                    f.log_path.display()
                );
            }
        }
        bail!("some tests failed");
    }

    Ok(())
}

fn write_log(
    log_dir: &Path,
    suite: &str,
    test_name: &str,
    stdout: &str,
    stderr: &str,
    cmd: &Command,
) -> Result<PathBuf> {
    let suite_dir = log_dir.join(sanitize(suite));
    fs::create_dir_all(&suite_dir)
        .with_context(|| format!("failed to create suite log dir {:?}", suite_dir))?;

    let file = suite_dir.join(format!("{}.log", sanitize(test_name)));
    let mut content = String::new();

    content.push_str("# command\n");
    content.push_str(&format!("{:?}\n\n", cmd));
    content.push_str("# stdout\n");
    content.push_str(stdout);
    if !stdout.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("\n# stderr\n");
    content.push_str(stderr);
    if !stderr.ends_with('\n') {
        content.push('\n');
    }

    fs::write(&file, content).with_context(|| format!("failed to write log {:?}", file))?;
    Ok(file)
}

fn sanitize(s: &str) -> String {
    // Keep it simple: replace problematic path chars with '_'
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn build_suite(riscv_tests: &Path, prefix: &str, suite: &str) -> Result<()> {
    run(Command::new("make")
        .arg("-C")
        .arg(riscv_tests)
        .arg(format!("RISCV_PREFIX={prefix}"))
        .arg(suite))
}

fn discover_tests(isa_dir: &Path, suite: &str, filter: Option<&str>) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let prefix = format!("{suite}-");

    for ent in WalkDir::new(isa_dir).min_depth(1).max_depth(1) {
        let ent = ent?;
        if !ent.file_type().is_file() {
            continue;
        }
        let p = ent.path();
        let name = p.file_name().and_then(OsStr::to_str).unwrap_or("");
        if !name.starts_with(&prefix) {
            continue;
        }
        // Exclude common non-ELF artifacts
        if name.ends_with(".dump") || name.ends_with(".hex") || name.ends_with(".objdump") {
            continue;
        }
        if let Some(f) = filter {
            if !name.contains(f) {
                continue;
            }
        }
        out.push(p.to_path_buf());
    }

    out.sort();
    Ok(out)
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("failed to run: {:?}", cmd))?;
    if !status.success() {
        return Err(anyhow!("command failed: {:?} -> {:?}", cmd, status));
    }
    Ok(())
}
