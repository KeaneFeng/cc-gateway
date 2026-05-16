use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};

const PID_FILE: &str = "~/.cc-gateway/cc-gateway.pid";
const LOG_FILE: &str = "~/.cc-gateway/cc-gateway.log";
const DEFAULT_PORT: u16 = 16789;

fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn pid_file_path() -> PathBuf {
    expand_path(PID_FILE)
}

fn log_file_path() -> PathBuf {
    expand_path(LOG_FILE)
}

fn read_pid() -> Result<Option<u32>> {
    let path = pid_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read PID file: {}", path.display()))?;
    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in file: {}", content.trim()))?;
    Ok(Some(pid))
}

fn write_pid(pid: u32) -> Result<()> {
    let path = pid_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", path.display()))
}

fn remove_pid_file() {
    let path = pid_file_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
}

fn is_process_running(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_running() -> Result<Option<u32>> {
    if let Some(pid) = read_pid()? {
        if is_process_running(pid) {
            return Ok(Some(pid));
        }
        remove_pid_file();
    }
    Ok(None)
}

/// Check if the port is already in use by trying to bind to it.
fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
}

/// Kill a process by PID with SIGTERM, wait up to 5s, then SIGKILL if needed.
fn kill_process(pid: u32, label: &str) -> Result<()> {
    if !is_process_running(pid) {
        return Ok(());
    }

    println!("  Sending SIGTERM to {} (PID: {})...", label, pid);
    std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()
        .context("Failed to send SIGTERM")?;

    for _ in 0..50 {
        if !is_process_running(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("  Force killing {} (PID: {})...", label, pid);
    std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status()
        .context("Failed to send SIGKILL")?;

    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

/// Start the server.
///
/// In foreground mode (default): writes PID file, then `exec`s into `cc-gateway serve`.
/// In daemon mode (`--daemon`): spawns `cc-gateway serve` in background, writes child PID.
/// If `force` is true, stops any existing server before starting.
pub fn run_start(
    config: &str,
    port: Option<u16>,
    host: Option<String>,
    daemon: bool,
    force: bool,
) -> Result<()> {
    let effective_port = port.unwrap_or(DEFAULT_PORT);

    if force {
        // Stop any running instance
        if check_running()?.is_some() || is_port_in_use(effective_port) {
            println!("Stopping existing cc-gateway...");
            run_stop()?;
            for _ in 0..30 {
                if !is_port_in_use(effective_port) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    } else if let Some(pid) = check_running()? {
        println!("cc-gateway is already running (PID: {})", pid);
        println!("  Use 'cc-gateway restart' or 'cc-gateway start --force' to restart");
        return Ok(());
    }

    let effective_port = port.unwrap_or(DEFAULT_PORT);

    // Check if port is already in use by something else
    if is_port_in_use(effective_port) {
        eprintln!("Error: port {} is already in use", effective_port);
        eprintln!("  Use 'cc-gateway stop' or 'cc-gateway restart' to take over the port");
        anyhow::bail!("port {} is already in use", effective_port);
    }

    let config_path = expand_path(config);
    let mut args = vec![
        "serve".to_string(),
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
    ];
    if let Some(p) = port {
        args.push("--port".to_string());
        args.push(p.to_string());
    }
    if let Some(h) = host {
        args.push("--host".to_string());
        args.push(h);
    }

    let exe = std::env::current_exe().context("Failed to get current executable path")?;

    if daemon {
        let log_path = log_file_path();
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Truncate log file on fresh start
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;

        let child = Command::new(&exe)
            .args(&args)
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to start cc-gateway in background")?;

        write_pid(child.id())?;

        // Wait up to 3s for the server to actually bind the port
        let start = std::time::Instant::now();
        loop {
            if !is_process_running(child.id()) {
                remove_pid_file();
                let log_tail = std::fs::read_to_string(&log_path)
                    .unwrap_or_else(|_| "(unable to read log)".to_string());
                anyhow::bail!(
                    "cc-gateway failed to start.\nLast log output:\n{}",
                    log_tail.trim()
                );
            }
            if !is_port_in_use(effective_port) {
                // Port not bound yet, keep waiting
                if start.elapsed() > Duration::from_secs(5) {
                    remove_pid_file();
                    let _ = kill_process(child.id(), "cc-gateway");
                    anyhow::bail!(
                        "cc-gateway timed out starting (5s). Check log: {}",
                        log_path.display()
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            // Port is bound — server is ready
            break;
        }

        println!(
            "cc-gateway started (PID: {}, port: {})",
            child.id(),
            effective_port
        );
        println!("  Log: {}", log_path.display());
        println!("  Stop: cc-gateway stop");
        Ok(())
    } else {
        // Foreground: exec replaces this process. PID stays the same.
        write_pid(std::process::id())?;
        let err = Command::new(&exe).args(&args).exec();
        remove_pid_file();
        Err(anyhow::anyhow!("Failed to exec cc-gateway: {}", err))
    }
}

/// Stop the running server gracefully (SIGTERM, then SIGKILL after 5s).
pub fn run_stop() -> Result<()> {
    let pid = match read_pid()? {
        Some(pid) => pid,
        None => {
            // No PID file — try to find process by port
            println!("cc-gateway is not running (no PID file)");
            if is_port_in_use(DEFAULT_PORT) {
                println!(
                    "  Port {} is in use, trying to find process...",
                    DEFAULT_PORT
                );
                if let Some(pid) = find_pid_by_port(DEFAULT_PORT) {
                    println!("  Found cc-gateway on port {} (PID: {})", DEFAULT_PORT, pid);
                    kill_process(pid, "cc-gateway")?;
                    println!("cc-gateway stopped");
                } else {
                    println!(
                        "  Could not find cc-gateway process on port {}",
                        DEFAULT_PORT
                    );
                    println!("  Kill manually: lsof -i :{} -t | xargs kill", DEFAULT_PORT);
                }
            }
            return Ok(());
        }
    };

    if !is_process_running(pid) {
        println!("cc-gateway is not running (stale PID file for {})", pid);
        remove_pid_file();
        return Ok(());
    }

    println!("Stopping cc-gateway (PID: {})...", pid);
    kill_process(pid, "cc-gateway")?;
    remove_pid_file();
    println!("cc-gateway stopped");
    Ok(())
}

/// Find PID of cc-gateway server process on a given port using lsof
/// Only matches LISTEN state (server), not connected clients (Claude Code)
fn find_pid_by_port(port: u16) -> Option<u32> {
    let output = std::process::Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-sTCP:LISTEN", "-t"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().lines().next()?.trim().parse::<u32>().ok()
}

/// Restart: stop + start.
pub fn run_restart(
    config: &str,
    port: Option<u16>,
    host: Option<String>,
    daemon: bool,
) -> Result<()> {
    let effective_port = port.unwrap_or(DEFAULT_PORT);

    // Always do a full stop — handles PID file, stale files, and orphan processes
    let was_running = check_running()?.is_some();
    if was_running || is_port_in_use(effective_port) {
        println!("Restarting cc-gateway...");
        run_stop()?;
        // Wait for port to be released (up to 5 seconds)
        for _ in 0..50 {
            if !is_port_in_use(effective_port) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if is_port_in_use(effective_port) {
            anyhow::bail!(
                "Port {} is still in use after stopping. Please check manually.",
                effective_port
            );
        }
    }

    run_start(config, port, host, daemon, false)
}

/// Show whether the server is running.
#[allow(dead_code)]
pub fn run_server_status() -> Result<()> {
    match check_running()? {
        Some(pid) => {
            println!("cc-gateway is running (PID: {})", pid);
            println!("  PID file: {}", pid_file_path().display());
            println!("  Log: {}", log_file_path().display());
        }
        None => {
            println!("cc-gateway is not running");
        }
    }
    Ok(())
}
