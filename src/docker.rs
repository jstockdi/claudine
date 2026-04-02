use std::io::Write;
use std::process::Command;

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
