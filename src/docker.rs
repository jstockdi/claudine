use std::io::{IsTerminal, Write};
use std::process::Command;

use crate::{config, project};

const DOCKERFILE: &str = include_str!("../Dockerfile");
const ENTRYPOINT: &str = include_str!("../entrypoint.sh");

/// Verify that Docker is installed and the daemon is running.
pub fn check_docker() -> anyhow::Result<()> {
    which::which("docker")
        .map_err(|_| anyhow::anyhow!("Docker not found on PATH. Please install Docker first."))?;

    let status = Command::new("docker")
        .arg("info")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run 'docker info': {e}"))?;

    if !status.success() {
        anyhow::bail!("Docker daemon is not running. Please start Docker and try again.");
    }

    Ok(())
}

/// Build the claudine Docker image.
pub fn cmd_build() -> anyhow::Result<()> {
    check_docker()?;

    let tmp = tempfile::tempdir()
        .map_err(|e| anyhow::anyhow!("Failed to create temporary directory: {e}"))?;

    // Write the Dockerfile into the temp directory
    let dockerfile_path = tmp.path().join("Dockerfile");
    let mut f = std::fs::File::create(&dockerfile_path)
        .map_err(|e| anyhow::anyhow!("Failed to write Dockerfile: {e}"))?;
    f.write_all(DOCKERFILE.as_bytes())?;

    // Write the entrypoint script into the temp directory
    let entrypoint_path = tmp.path().join("entrypoint.sh");
    let mut f = std::fs::File::create(&entrypoint_path)
        .map_err(|e| anyhow::anyhow!("Failed to write entrypoint.sh: {e}"))?;
    f.write_all(ENTRYPOINT.as_bytes())?;

    println!("Building claudine Docker image...");

    let status = Command::new("docker")
        .args(["build", "-t", "claudine:latest"])
        .arg(tmp.path())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run 'docker build': {e}"))?;

    if !status.success() {
        anyhow::bail!("Docker build failed with exit code: {}", status);
    }

    println!("Successfully built claudine:latest");
    Ok(())
}

/// Launch Claude Code in a container for the given project.
///
/// Validates the project state, assembles Docker run arguments, and replaces the
/// current process with the Docker container via `exec()`.
pub fn cmd_run(project: &str, extra_args: &[String]) -> anyhow::Result<()> {
    project::validate_name(project)?;

    if !project::volume_exists(project)? {
        anyhow::bail!(
            "Project '{}' is not initialized. Run 'claudine init {}' first.",
            project,
            project
        );
    }

    if project::container_running(project)? {
        anyhow::bail!(
            "Project '{}' is already running. Use 'claudine shell {}' to open another terminal.",
            project,
            project
        );
    }

    let project_config = config::load_project(project)?;
    let global_config = config::load_global()?;
    let image = config::resolve_image(&project_config, &global_config);

    let docker_args = build_run_args(project, &image);

    // Build the full command: docker <run_args> <image> claude [extra_args...]
    let mut cmd = Command::new("docker");
    cmd.args(&docker_args);

    // Command to run inside the container
    cmd.arg("claude");
    cmd.args(extra_args);

    // Replace the current process with Docker. This call only returns on error.
    use std::os::unix::process::CommandExt;
    let err = cmd.exec();
    Err(anyhow::anyhow!("Failed to exec docker: {}", err))
}

/// Assemble Docker run arguments for launching a project container.
///
/// This function is shared between `cmd_run` and `cmd_shell` to ensure
/// consistent container configuration.
pub(crate) fn build_run_args(project: &str, image: &str) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--name".to_string(),
        project::container_name(project),
        "-v".to_string(),
        format!("{}:/workspace", project::volume_name(project)),
        "-v".to_string(),
        "/var/run/docker.sock:/var/run/docker.sock".to_string(),
    ];

    // Mount host gitconfig if it exists
    if let Some(home) = dirs::home_dir() {
        let gitconfig = home.join(".gitconfig");
        if gitconfig.exists() {
            args.push("-v".to_string());
            args.push(format!("{}:/host-config/gitconfig:ro", gitconfig.display()));
        }

        let ssh_dir = home.join(".ssh");
        if ssh_dir.exists() {
            args.push("-v".to_string());
            args.push(format!("{}:/host-config/ssh:ro", ssh_dir.display()));
        }

        let claude_dir = home.join(".claude");
        if claude_dir.exists() {
            args.push("-v".to_string());
            args.push(format!(
                "{}:/host-config/claude-credentials:ro",
                claude_dir.display()
            ));
        }
    }

    args.push("-w".to_string());
    args.push("/workspace/project".to_string());

    args.push("-e".to_string());
    args.push("HOME=/workspace/home".to_string());

    args.push("--shm-size=256m".to_string());

    // Pass through ANTHROPIC_API_KEY if set in the host environment
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        args.push("-e".to_string());
        args.push("ANTHROPIC_API_KEY".to_string());
    }

    // Only allocate a TTY if stdin is a terminal
    if std::io::stdin().is_terminal() {
        args.push("-it".to_string());
    }

    // Image name is the last positional arg before the command
    args.push(image.to_string());

    args
}
