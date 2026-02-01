use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    ffi::OsStr,
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

    /// Which ISA subset to run (e.g. rv64ui-p, rv64mi-p, rv64si-p)
    #[arg(long, default_value = "rv64ui-p")]
    suite: String,

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
        suite,
        emulator,
        build,
        filter,
        timeout_sec: _,
        emu_args,
    } = args;

    // 1) Ensure emulator exists
    if !emulator.exists() {
        bail!(
            "emulator not found at {:?}. Build it (e.g. cargo build --release) or pass --emulator.",
            emulator
        );
    }

    // 2) Build riscv-tests
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
            "[xtask] building riscv-tests (suite={suite}) with RISCV_PREFIX={prefix} ..."
        );
        if build_suite(&riscv_tests, &prefix, &suite).is_err() {
            eprintln!(
                "[xtask] suite target build failed; falling back to `make -C riscv-tests isa` ..."
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

    // 3) Discover tests
    let isa_dir = riscv_tests.join("isa");
    if !isa_dir.exists() {
        bail!("expected riscv-tests build dir at {:?}. Build may have failed.", isa_dir);
    }

    let tests = discover_tests(&isa_dir, &suite, filter.as_deref())?;
    if tests.is_empty() {
        bail!("no tests found under {:?} for suite={suite}", isa_dir);
    }

    eprintln!("[xtask] discovered {} tests", tests.len());

    // 4) Run tests
    let mut passed = 0usize;
    let mut failed = Vec::new();

    for t in &tests {
        let name = t.file_name().and_then(OsStr::to_str).unwrap_or("<nonutf8>");
        eprint!("[xtask] RUN  {name} ... ");

        let mut cmd = Command::new(&emulator);

        // xtask controls which test ELF is executed
        cmd.arg("--elf").arg(t);

        // pass user-specified args verbatim
        for a in &emu_args {
            cmd.arg(a);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let out = cmd
            .output()
            .with_context(|| format!("failed to spawn emulator for {name}"))?;

        if out.status.success() {
            passed += 1;
            eprintln!("ok");
        } else {
            let code = out.status.code().unwrap_or(-1);
            eprintln!("FAIL (code={code})");

            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);

            failed.push((t.clone(), code, stdout.into_owned(), stderr.into_owned()));
        }
    }

    // 5) Summary
    eprintln!("\n[xtask] summary: passed={passed}, failed={}", failed.len());
    if !failed.is_empty() {
        eprintln!("[xtask] failures:");
        for (path, code, stdout, stderr) in &failed {
            let name = path.file_name().and_then(OsStr::to_str).unwrap_or("<nonutf8>");
            eprintln!("--- {name} (code={code}) ---");
            if !stdout.trim().is_empty() {
                eprintln!("[stdout]\n{stdout}");
            }
            if !stderr.trim().is_empty() {
                eprintln!("[stderr]\n{stderr}");
            }
        }
        bail!("some tests failed");
    }

    Ok(())
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
