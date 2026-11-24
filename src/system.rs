use std::path::Path;

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use serde::Serialize;

use crate::cli::{Cli, OutputFormat, ShellType};
use crate::error::AetherError;

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub checks: Vec<DoctorCheck>,
}

pub fn run_doctor(data_dir: &Path, format: OutputFormat, json_flag: bool) -> Result<(), AetherError> {
    let mut checks = Vec::new();

    let data_dir_exists = data_dir.exists();
    let data_dir_writable = if data_dir_exists {
        std::fs::metadata(data_dir)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    } else {
        true
    };

    checks.push(DoctorCheck {
        name: "data_dir".to_string(),
        status: if data_dir_writable { "ok" } else { "warn" }.to_string(),
        detail: format!(
            "{} (exists={}, writable={})",
            data_dir.display(),
            data_dir_exists,
            data_dir_writable
        ),
    });

    checks.push(DoctorCheck {
        name: "offline_mode".to_string(),
        status: "ok".to_string(),
        detail: "local fixture and offline analysis supported without internet".to_string(),
    });

    checks.push(DoctorCheck {
        name: "packet_capture".to_string(),
        status: "info".to_string(),
        detail: "unprivileged mode active; live interface capture requires Phase 3 adapter"
            .to_string(),
    });

    let report = DoctorReport {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: data_dir.display().to_string(),
        checks,
    };

    if json_flag || matches!(format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("AetherLattice System Doctor");
        println!("OS: {} ({})", report.os, report.arch);
        println!("Data Dir: {}", report.data_dir);
        println!("{:-<60}", "");
        for c in &report.checks {
            println!("[{:>4}] {:<18} {}", c.status.to_uppercase(), c.name, c.detail);
        }
    }

    Ok(())
}

pub fn run_info(json_flag: bool) -> Result<(), AetherError> {
    let info = serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    if json_flag {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!("Platform: {}/{}", std::env::consts::OS, std::env::consts::ARCH);
    }
    Ok(())
}

pub fn run_completion(shell: ShellType) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    let target_shell = match shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Fish => Shell::Fish,
        ShellType::Powershell => Shell::PowerShell,
    };
    generate(target_shell, &mut cmd, bin_name, &mut std::io::stdout());
}
