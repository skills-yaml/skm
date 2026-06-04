use std::process::Command;

const REPOSITORY: &str = "skills-yaml/skm";
const REPOSITORY_URL: &str = "https://github.com/skills-yaml/skm.git";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateChannel {
    Prod,
    Development,
}

impl UpdateChannel {
    pub fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "prod" | "production" => Ok(Self::Prod),
            "dev" | "development" => Ok(Self::Development),
            _ => Err(format!("Unsupported update channel '{}'", value).into()),
        }
    }

    pub fn as_installer_arg(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::Development => "development",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Prod => "prod-latest",
            Self::Development => "development-latest",
        }
    }
}

pub fn current_build_commit() -> &'static str {
    env!("SKM_BUILD_COMMIT")
}

pub fn current_build_channel() -> &'static str {
    env!("SKM_BUILD_CHANNEL")
}

pub fn check_for_update(channel: UpdateChannel) -> Result<bool, Box<dyn std::error::Error>> {
    let current = current_build_commit();
    let latest = latest_release_commit(channel)?;
    let current_short = short_sha(current);
    let latest_short = short_sha(&latest);

    println!(
        "Current build: {} ({})",
        current_short,
        current_build_channel()
    );
    println!("Latest {} build: {}", channel.tag(), latest_short);

    if current == "unknown" {
        println!("Update status: unknown (current build commit is not embedded)");
        return Ok(true);
    }

    if current == latest {
        println!("Update status: up to date");
        Ok(false)
    } else {
        println!("Update status: update available");
        Ok(true)
    }
}

pub fn install_update(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(windows) {
        install_update_windows(channel)
    } else {
        install_update_unix(channel)
    }
}

fn latest_release_commit(channel: UpdateChannel) -> Result<String, Box<dyn std::error::Error>> {
    let ref_name = format!("refs/tags/{}", channel.tag());
    let output = Command::new("git")
        .args(["ls-remote", REPOSITORY_URL, &ref_name])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to check latest release tag: {}", err).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let commit = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("Release tag '{}' was not found", channel.tag()))?;

    Ok(commit.to_string())
}

fn install_update_unix(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    let script_url = format!(
        "https://raw.githubusercontent.com/{}/main/scripts/install.sh",
        REPOSITORY
    );
    let command = format!(
        "curl -fsSL '{}' | sh -s -- {}",
        script_url,
        channel.as_installer_arg()
    );

    println!("Running installer for {}...", channel.tag());
    let status = Command::new("sh").args(["-c", &command]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Installer exited with status {}", status).into())
    }
}

#[cfg(windows)]
fn install_update_windows(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    let script_url = format!(
        "https://raw.githubusercontent.com/{}/main/scripts/install.ps1",
        REPOSITORY
    );
    let command = format!(
        "Start-Sleep -Seconds 2; $script = Join-Path $env:TEMP 'skm-install.ps1'; Invoke-WebRequest -Uri '{}' -OutFile $script; & $script -Channel {} -AddToPath",
        script_url,
        channel.as_installer_arg()
    );

    println!("Starting Windows updater for {}...", channel.tag());
    println!("The update will continue in a separate PowerShell process after skm exits.");
    Command::new("cmd")
        .args([
            "/C",
            "start",
            "",
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .spawn()?;

    Ok(())
}

#[cfg(not(windows))]
fn install_update_windows(_channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    unreachable!("Windows updater is only used on Windows")
}

fn short_sha(value: &str) -> &str {
    if value.len() > 12 {
        &value[..12]
    } else {
        value
    }
}
