//! Migrate a project from the legacy single-volume layout to the new
//! bind-mount + home-volume layout.
//!
//! Steps:
//! 1. Copy everything under the legacy volume EXCEPT `home/` to the host bind
//!    directory (default: `~/projects/<project>/`).
//! 2. Copy the legacy volume's `home/` tree into a new named home volume
//!    (`claudine_<project>_home`).
//! 3. Rewrite the project config with `host_dir` set.
//! 4. Regenerate `devcontainer.json` under the new host directory.
//!
//! The legacy volume is left intact for safety; remove manually with
//! `docker volume rm claudine_<project>` after verification.

use std::path::Path;
use std::process::Command;

use dialoguer::Confirm;

use crate::{config, devcontainer, project};

/// Run the migration for a single project.
pub fn cmd_migrate(project_name: &str, assume_yes: bool) -> anyhow::Result<()> {
    project::validate_name(project_name)?;

    let mut project_config = config::load_project(project_name)?;

    if project_config.host_dir.is_some() {
        anyhow::bail!(
            "Project '{}' is already migrated (host_dir is set).",
            project_name
        );
    }

    if !project::volume_exists(project_name)? {
        anyhow::bail!(
            "Legacy volume '{}' does not exist — nothing to migrate.",
            project::volume_name(project_name)
        );
    }

    let host_dir = project::default_host_dir(project_name)?;
    let home_volume = project::home_volume_name(project_name);
    let legacy_volume = project::volume_name(project_name);

    println!("Migration plan for '{}':", project_name);
    println!("  Legacy volume:  {}", legacy_volume);
    println!("  New host dir:   {}", host_dir.display());
    println!("  New home vol:   {}", home_volume);
    println!();
    println!("Repos and non-home files will be copied from the legacy volume to");
    println!("the host directory. /project/home will be copied into the new home volume.");
    println!("The legacy volume will be preserved until you manually remove it.");
    println!();

    if !assume_yes {
        let confirmed = Confirm::new()
            .with_prompt("Proceed?")
            .default(true)
            .interact()?;
        if !confirmed {
            anyhow::bail!("Migration cancelled.");
        }
    }

    // 1. Create host directory
    if !host_dir.exists() {
        std::fs::create_dir_all(&host_dir).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create host directory '{}': {e}",
                host_dir.display()
            )
        })?;
        println!("Created host directory: {}", host_dir.display());
    } else if host_dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
        if !assume_yes {
            let proceed = Confirm::new()
                .with_prompt(format!(
                    "Host directory {} is not empty. Copy into it anyway?",
                    host_dir.display()
                ))
                .default(false)
                .interact()?;
            if !proceed {
                anyhow::bail!("Migration cancelled.");
            }
        }
    }

    // 2. Create the home volume (idempotent)
    project::docker_volume_create(&home_volume)?;

    // 3. Copy repos out of legacy volume to host dir (exclude home/)
    println!("Copying repos from volume to host dir...");
    copy_volume_repos_to_host(&legacy_volume, &host_dir)?;

    // 4. Copy /project/home into the new home volume
    println!("Copying home contents into new home volume...");
    copy_volume_home_to_home_volume(&legacy_volume, &home_volume)?;

    // 5. Update project config
    project_config.host_dir = Some(host_dir.to_string_lossy().to_string());
    config::save_project(project_name, &project_config)?;
    println!("Updated project config.");

    // 6. Regenerate devcontainer.json in the new location
    match devcontainer::write(project_name, None) {
        Ok(path) => println!("Wrote devcontainer.json: {}", path.display()),
        Err(e) => eprintln!("Warning: failed to regenerate devcontainer.json: {e}"),
    }

    println!();
    println!("Migration complete for '{}'.", project_name);
    println!("Legacy volume '{}' preserved. After verifying everything works,", legacy_volume);
    println!("remove it with: docker volume rm {}", legacy_volume);

    Ok(())
}

/// Copy all directories/files at the volume root EXCEPT `home/` into `dest`.
///
/// Uses `rsync -a` via a one-shot container that mounts the legacy volume
/// read-only and the host destination as a bind mount.
fn copy_volume_repos_to_host(volume: &str, dest: &Path) -> anyhow::Result<()> {
    // Use busybox/alpine for a lean copy container that's always available locally.
    // `cp -a` preserves symlinks and timestamps; --reflink isn't available but
    // performance for a one-shot migration is fine.
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/src:ro", volume),
            "-v",
            &format!("{}:/dst", dest.display()),
            "alpine:3.19",
            "sh",
            "-c",
            // Copy everything except home/
            "cd /src && for entry in * .[!.]* ..?*; do \
                [ -e \"$entry\" ] || continue; \
                [ \"$entry\" = home ] && continue; \
                cp -a \"$entry\" /dst/; \
             done",
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run copy container: {e}"))?;

    if !status.success() {
        anyhow::bail!("Failed to copy repos from volume '{}' to {}", volume, dest.display());
    }
    Ok(())
}

/// Copy `/project/home` contents from the legacy volume into the new home volume.
fn copy_volume_home_to_home_volume(src_volume: &str, home_volume: &str) -> anyhow::Result<()> {
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/src:ro", src_volume),
            "-v",
            &format!("{}:/dst", home_volume),
            "alpine:3.19",
            "sh",
            "-c",
            "if [ -d /src/home ]; then cp -a /src/home/. /dst/; fi",
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run copy container: {e}"))?;

    if !status.success() {
        anyhow::bail!(
            "Failed to copy home contents from volume '{}' to '{}'",
            src_volume, home_volume
        );
    }
    Ok(())
}
