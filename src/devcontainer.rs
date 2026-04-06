use std::process::Command;

use crate::{config, project};

/// Generate devcontainer.json content for a project.
pub fn generate(project: &str, repo: Option<&str>) -> anyhow::Result<String> {
    let project_config = config::load_project(project)?;
    let global_config = config::load_global()?;
    let image = config::resolve_image(&project_config, &global_config);

    let workspace_folder = match repo {
        Some(r) => format!("/project/{}", r),
        None => "/project".to_string(),
    };

    let json = serde_json::json!({
        "name": project::container_name(project),
        "image": image,
        "overrideCommand": true,
        "remoteUser": "claude",
        "workspaceMount": format!(
            "source={},target=/project,type=volume",
            project::volume_name(project)
        ),
        "workspaceFolder": workspace_folder,
        "mounts": [
            "source=/var/run/docker.sock,target=/var/run/docker.sock,type=bind"
        ],
        "runArgs": ["--shm-size=256m"],
        "containerEnv": {
            "HOME": "/project/home"
        }
    });

    serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow::anyhow!("Failed to serialize devcontainer.json: {e}"))
}

/// Write devcontainer.json into the project's share directory.
/// Returns the path to the written file.
pub fn write(project: &str, repo: Option<&str>) -> anyhow::Result<std::path::PathBuf> {
    let share = project::share_dir(project)?;
    if !share.exists() {
        anyhow::bail!(
            "Share directory does not exist: {}. Run 'claudine init {}' first.",
            share.display(),
            project
        );
    }

    let devcontainer_dir = share.join(".devcontainer");
    std::fs::create_dir_all(&devcontainer_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create .devcontainer directory: {e}"))?;

    let path = devcontainer_dir.join("devcontainer.json");
    let content = generate(project, repo)?;
    std::fs::write(&path, &content)
        .map_err(|e| anyhow::anyhow!("Failed to write devcontainer.json: {e}"))?;

    Ok(path)
}

/// Generate devcontainer.json and open the project in Zed.
pub fn cmd_zed(project: &str, repo: Option<&str>) -> anyhow::Result<()> {
    let path = write(project, repo)?;
    println!("Generated {}", path.display());

    let share = project::share_dir(project)?;

    match which::which("zed") {
        Ok(_) => {
            let status = Command::new("zed")
                .arg(&share)
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to launch Zed: {e}"))?;

            if !status.success() {
                eprintln!("Zed exited with non-zero status. Open manually:");
                println!("  zed {}", share.display());
            }
        }
        Err(_) => {
            println!("Zed not found on PATH. Open this directory in Zed:");
            println!("  zed {}", share.display());
        }
    }

    Ok(())
}
