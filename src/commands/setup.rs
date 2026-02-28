use anyhow::{bail, Context, Result};
use colored::*;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

use crate::config::{self, DevConfig};

use super::GITHUB_REPO;

/// Setup command: download and install SDK
pub async fn setup(version: Option<String>) -> Result<()> {
    println!("{}", "MPF SDK Setup".bold().cyan());

    let version = match version {
        Some(v) => v,
        None => {
            println!("Fetching latest release...");
            fetch_latest_version().await?
        }
    };

    let version_normalized = if version.starts_with('v') {
        version.clone()
    } else {
        format!("v{}", version)
    };

    println!("Installing SDK version: {}", version_normalized.green());

    let sdk_root = config::sdk_root();
    let version_dir = config::version_dir(&version_normalized);

    // Check if already installed
    if version_dir.exists() {
        println!(
            "{} Version {} is already installed",
            "Note:".yellow(),
            version_normalized
        );
    } else {
        // Download and extract
        download_and_extract(&version_normalized, &version_dir).await?;
    }

    // Set as current
    config::set_current_version(&version_normalized)?;

    // Update dev.json
    let mut dev_config = DevConfig::load().unwrap_or_default();
    dev_config.sdk_version = Some(version_normalized.clone());
    dev_config.save()?;

    println!(
        "{} SDK {} installed and set as current",
        "✓".green(),
        version_normalized
    );
    println!("  Location: {}", sdk_root.display());

    Ok(())
}

async fn fetch_latest_version() -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "mpf-dev")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    resp["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .context("Could not find latest release")
}

async fn download_and_extract(version: &str, dest: &std::path::PathBuf) -> Result<()> {
    // Determine platform and asset name
    let (asset_name, is_tarball) = if cfg!(target_os = "windows") {
        ("mpf-windows-x64.zip".to_string(), false)
    } else {
        ("mpf-linux-x64.tar.gz".to_string(), true)
    };

    let download_url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, version, asset_name
    );

    println!("Downloading {} ({})...", asset_name, version);

    let client = reqwest::Client::new();
    let resp = client
        .get(&download_url)
        .header("User-Agent", "mpf-dev")
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!(
            "Failed to download SDK: {} ({})",
            resp.status(),
            download_url
        );
    }

    let total_size = resp.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );

    // Download to temp file
    let temp_ext = if is_tarball { "tar.gz.tmp" } else { "zip.tmp" };
    let temp_path = dest.with_extension(temp_ext);
    if let Some(parent) = temp_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&temp_path)?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Downloaded");

    // Extract
    println!("Extracting...");
    fs::create_dir_all(dest)?;

    if is_tarball {
        // Extract tar.gz using tar command (more reliable on Unix)
        let status = Command::new("tar")
            .args([
                "-xzf",
                &temp_path.to_string_lossy(),
                "-C",
                &dest.to_string_lossy(),
            ])
            .status()
            .context("Failed to run tar command")?;
        if !status.success() {
            bail!("tar extraction failed");
        }
    } else {
        // Extract zip
        let file = File::open(&temp_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(dest)?;
    }

    // Clean up temp file
    fs::remove_file(&temp_path)?;

    println!("{} Extraction complete", "✓".green());
    Ok(())
}

/// Versions command: list installed versions
pub fn versions() -> Result<()> {
    let versions = config::installed_versions();
    let current = config::current_version();

    if versions.is_empty() {
        println!("No SDK versions installed.");
        println!("Run {} to install.", "mpf-dev setup".cyan());
        return Ok(());
    }

    println!("{}", "Installed SDK versions:".bold());
    for v in &versions {
        if Some(v) == current.as_ref() {
            println!("  {} {} {}", "*".green(), v.green(), "(current)".dimmed());
        } else {
            println!("    {}", v);
        }
    }

    Ok(())
}

/// Use command: switch SDK version
pub fn use_version(version: &str) -> Result<()> {
    let version_normalized = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    };

    let version_dir = config::version_dir(&version_normalized);

    if !version_dir.exists() {
        bail!(
            "Version {} is not installed. Run `mpf-dev setup --version {}`",
            version_normalized,
            version
        );
    }

    config::set_current_version(&version_normalized)?;

    // Update dev.json
    let mut dev_config = DevConfig::load().unwrap_or_default();
    dev_config.sdk_version = Some(version_normalized.clone());
    dev_config.save()?;

    println!(
        "{} Now using SDK {}",
        "✓".green(),
        version_normalized
    );

    Ok(())
}
