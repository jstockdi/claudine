use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

use dialoguer::{Confirm, Input};

use crate::{config, plugin, project};

/// Initialize a project. If `flag_repos` are provided, runs non-interactively.
/// Otherwise, prompts for input (requires a TTY).
pub fn cmd_init(name: &str, flag_ssh_key: Option<&str>, flag_repos: &[String], flag_plugins: &[String]) -> anyhow::Result<()> {
    project::validate_name(name)?;

    let interactive = flag_repos.is_empty();

    if interactive && !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "No TTY detected. Use --repo and --ssh-key flags for non-interactive init.\n\
             Example: claudine init {} --ssh-key ~/.ssh/id_rsa --repo git@github.com:user/repo.git",
            name
        );
    }

    // Check if volume already exists
    let volume_already_exists = project::volume_exists(name)?;
    if volume_already_exists && interactive {
        let proceed = Confirm::new()
            .with_prompt(format!(
                "Volume already exists for '{}'. Re-initialize? This will not delete existing data.",
                name
            ))
            .default(false)
            .interact()?;

        if !proceed {
            anyhow::bail!("Init cancelled.");
        }
    }

    // Resolve SSH key
    let ssh_key = if let Some(key) = flag_ssh_key {
        let path = PathBuf::from(key);
        if !path.exists() {
            anyhow::bail!("SSH key not found: {}", path.display());
        }
        Some(path.display().to_string())
    } else if interactive {
        let ssh_key_input: String = Input::new()
            .with_prompt("SSH key path (leave empty for HTTPS repos)")
            .default(String::new())
            .show_default(false)
            .interact_text()?;

        if ssh_key_input.trim().is_empty() {
            None
        } else {
            let path = PathBuf::from(ssh_key_input.trim());
            if !path.exists() {
                anyhow::bail!("SSH key not found: {}", path.display());
            }
            Some(path.display().to_string())
        }
    } else {
        None
    };

    // Validate plugins upfront
    for p in flag_plugins {
        if plugin::find(p).is_none() {
            anyhow::bail!(
                "Unknown plugin '{}'. Run 'claudine plugin available' to see options.",
                p
            );
        }
    }
    // Check plugin requirements (order matters — check in order they're given)
    for (i, p) in flag_plugins.iter().enumerate() {
        let installed_so_far: Vec<String> = flag_plugins[..i].to_vec();
        plugin::check_requires(p, &installed_so_far)?;
    }

    // Collect repos
    let repos = if interactive {
        collect_repos_interactive()?
    } else {
        collect_repos_from_flags(flag_repos)?
    };

    // Create volume if it does not already exist
    if !volume_already_exists {
        println!("Creating volume '{}'...", project::volume_name(name));
        project::create_volume(name)?;
    }

    // Create shared directory on the host
    let share_dir = project::share_dir(name)?;
    if !share_dir.exists() {
        std::fs::create_dir_all(&share_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create share directory '{}': {e}", share_dir.display()))?;
        println!("Created share directory: {}", share_dir.display());
    }

    // Build and save project config
    let plugins = if flag_plugins.is_empty() {
        None
    } else {
        Some(flag_plugins.to_vec())
    };
    let image_override = if plugins.is_some() {
        Some(config::ImageConfig { name: format!("claudine:{}", name) })
    } else {
        None
    };
    let project_config = config::ProjectConfig {
        repos: repos.clone(),
        ssh_key: ssh_key.clone(),
        plugins,
        image: image_override,
    };
    config::save_project(name, &project_config)?;

    // Resolve the image name from global config
    let global_config = config::load_global()?;
    let image = config::resolve_image(&project_config, &global_config);

    // Set up home directory with configs, credentials, and settings
    println!("Setting up home directory...");
    setup_home(name, &image, ssh_key.as_deref())?;

    // Clone each repo
    for repo in &repos {
        clone_repo(name, &image, repo)?;
    }

    // Build project image if plugins were specified
    if let Some(ref plugin_list) = project_config.plugins {
        if !plugin_list.is_empty() {
            let dockerfile = plugin::generate_dockerfile(plugin_list)?;
            crate::docker::cmd_build_project(name, &dockerfile)?;
        }
    }

    println!("Project '{}' initialized successfully.", name);
    Ok(())
}

/// Collect repos interactively via dialoguer prompts.
fn collect_repos_interactive() -> anyhow::Result<Vec<config::RepoConfig>> {
    let mut repos = Vec::new();

    loop {
        let url_input: String = Input::new()
            .with_prompt("Repository URL (leave empty to finish)")
            .default(String::new())
            .show_default(false)
            .interact_text()?;

        if url_input.trim().is_empty() {
            if repos.is_empty() {
                anyhow::bail!("At least one repository is required.");
            }
            break;
        }

        let url = url_input.trim().to_string();
        if url.starts_with('-') {
            anyhow::bail!("Repository URL cannot start with '-'.");
        }
        let default_dir = config::repo_dir_from_url(&url);

        let dir_input: String = Input::new()
            .with_prompt(format!("Directory name [{}]", default_dir))
            .default(default_dir.clone())
            .show_default(false)
            .interact_text()?;

        let dir = if dir_input.trim().is_empty() {
            default_dir
        } else {
            dir_input.trim().to_string()
        };
        project::validate_dir(&dir)?;

        let branch_input: String = Input::new()
            .with_prompt("Branch (leave empty for default)")
            .default(String::new())
            .show_default(false)
            .interact_text()?;

        let branch = if branch_input.trim().is_empty() {
            None
        } else {
            Some(branch_input.trim().to_string())
        };

        repos.push(config::RepoConfig { url, dir, branch });
    }

    Ok(repos)
}

/// Build repo configs from CLI flags. Dir is derived from URL, branch is always default.
fn collect_repos_from_flags(urls: &[String]) -> anyhow::Result<Vec<config::RepoConfig>> {
    if urls.is_empty() {
        anyhow::bail!("At least one --repo is required.");
    }

    let mut repos = Vec::new();
    for url in urls {
        if url.starts_with('-') {
            anyhow::bail!("Repository URL cannot start with '-'.");
        }
        let dir = config::repo_dir_from_url(url);
        project::validate_dir(&dir)?;
        repos.push(config::RepoConfig {
            url: url.clone(),
            dir,
            branch: None,
        });
    }

    Ok(repos)
}

const SETUP_HOME_SCRIPT: &str = include_str!("../setup-home.sh");

/// Set up the home directory in the volume with configs, credentials, and settings.
/// Runs a one-shot container with the embedded setup script.
fn setup_home(
    project_name: &str,
    image: &str,
    ssh_key: Option<&str>,
) -> anyhow::Result<()> {
    let volume = project::volume_name(project_name);
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    // Write setup script to a temp file
    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| anyhow::anyhow!("Failed to create temp file: {e}"))?;
    std::io::Write::write_all(&mut tmp, SETUP_HOME_SCRIPT.as_bytes())?;

    let mut args: Vec<String> = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-v".to_string(),
        format!("{}:/project", volume),
        "-v".to_string(),
        format!("{}:/tmp/setup-home.sh:ro", tmp.path().display()),
    ];

    // Mount host configs for the setup script to copy
    let gitconfig = home.join(".gitconfig");
    if gitconfig.exists() {
        args.extend(["-v".to_string(), format!("{}:/tmp/host-gitconfig:ro", gitconfig.display())]);
    }

    if let Some(key_path) = ssh_key {
        args.extend(["-v".to_string(), format!("{}:/tmp/host-ssh-key:ro", key_path)]);
    }

    args.extend([
        "--entrypoint".to_string(), "bash".to_string(),
        image.to_string(),
        "/tmp/setup-home.sh".to_string(),
    ]);

    let status = Command::new("docker")
        .args(&args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run setup container: {e}"))?;

    if !status.success() {
        anyhow::bail!("Home directory setup failed.");
    }

    Ok(())
}

/// Clone a single repository into the project volume.
/// Assumes setup_home has already run, so SSH keys and git config are in the volume.
pub fn clone_repo(
    project_name: &str,
    image: &str,
    repo: &config::RepoConfig,
) -> anyhow::Result<()> {
    let volume = project::volume_name(project_name);

    // Clone command — clone into /project/<dir>
    let clone_target = format!("/project/{}", repo.dir);
    let mut clone_cmd = vec!["git".to_string(), "clone".to_string()];
    if let Some(ref b) = repo.branch {
        clone_cmd.push("--branch".to_string());
        clone_cmd.push(b.clone());
    }
    clone_cmd.push("--".to_string());
    clone_cmd.push(repo.url.clone());
    clone_cmd.push(clone_target);

    let mut args: Vec<String> = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-v".to_string(),
        format!("{}:/project", volume),
        "-e".to_string(),
        "HOME=/project/home".to_string(),
        image.to_string(),
    ];
    args.extend(clone_cmd);

    println!("Cloning {}...", repo.dir);
    let status = Command::new("docker")
        .args(&args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run 'docker run' for clone: {e}"))?;

    if !status.success() {
        eprintln!(
            "Clone of '{}' failed (exit code: {}). The volume has been kept — \
             you can fix the issue and try again.",
            repo.url, status,
        );
        anyhow::bail!("Repository clone failed for '{}'.", repo.url);
    }

    Ok(())
}
