# Claudine Implementation Plan

> **Note:** Config paths are platform-specific. On macOS: `~/Library/Application Support/claudine/`. On Linux: `~/.config/claudine/`. Test scripts below use `$CONFIG_DIR` — set it with:
> ```bash
> # macOS
> CONFIG_DIR="$HOME/Library/Application Support/claudine"
> # Linux
> CONFIG_DIR="$HOME/.config/claudine"
> ```

Each step produces a working, testable artifact. Complete the tests before moving to the next step.

---

## Step 1: Dockerfile + Entrypoint

Create the container image and entrypoint script. This is the foundation — everything else wraps it.

**Files:**
- `Dockerfile`
- `entrypoint.sh`

**Dockerfile contents:**
- Base: `debian:bookworm`
- System packages: `ca-certificates curl gnupg gosu git python3 python3-pip vim`
- Docker CLI: add Docker apt repo, install `docker-ce-cli docker-compose-plugin`
- Claude Code CLI: `curl -fsSL https://claude.ai/install.sh | bash`, copy to `/usr/local/bin/claude`
- Non-root user: `useradd -m -d /home/claude -s /bin/bash claude`
- Alias: `echo 'alias claude="claude --dangerously-skip-permissions"' >> /etc/bash.bashrc`
- Copy entrypoint, set `WORKDIR /workspace`

**entrypoint.sh contents:**
- `mkdir -p /workspace/home /workspace/project`
- `chown claude:claude /workspace/home /workspace/project`
- `chown -R claude:claude /workspace/home`
- Copy from `/host-config/`: gitconfig, ssh, claude-credentials
- `gosu claude git config --global --add safe.directory '*'`
- Docker socket GID detection + `usermod -aG`
- `exec gosu claude "$@"` (or `bash` if no args)

**Test:**
```bash
# Build the image
docker build -t claudine:latest .

# Verify claude CLI is installed
docker run --rm claudine:latest claude --version

# Verify user drop works (should print "claude")
docker run --rm claudine:latest whoami

# Verify entrypoint creates workspace dirs
docker run --rm -v claudine_test:/workspace claudine:latest ls -la /workspace/
# expect: home/ and project/ owned by claude

# Verify git config bind mount works
docker run --rm \
  -v ~/.gitconfig:/host-config/gitconfig:ro \
  claudine:latest \
  git config --global user.name
# expect: your git username

# Verify SSH keys are copied with correct perms
docker run --rm \
  -v ~/.ssh:/host-config/ssh:ro \
  claudine:latest \
  bash -c 'ls -la ~/.ssh/ && stat -c "%a" ~/.ssh/id_*'
# expect: 600 permissions on key files

# Verify Docker socket access (DooD)
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  claudine:latest \
  docker ps
# expect: list of running containers (from host daemon)

# Cleanup
docker volume rm claudine_test
```

---

## Step 2: Rust Project Skeleton + `build` Command

Initialize the Rust project with clap and implement the first command: `claudine build`.

**Files:**
- `Cargo.toml`
- `src/main.rs` — clap app, command routing
- `src/cli.rs` — clap derive structs for all subcommands (stubs for unimplemented ones)
- `src/docker.rs` — embedded build assets, `cmd_build()` implementation

**Cargo.toml dependencies:**
```toml
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dialoguer = "0.11"
which = "7"
tempfile = "3"
anyhow = "1"
```

**cli.rs — define all subcommands upfront:**
```
Cli
├── Init { project: String }
├── Run { project: String, args: Vec<String> }
├── Shell { project: String }
├── Destroy { project: String }
├── Build
├── List
└── Completions { shell: clap_complete::Shell }
```

**docker.rs — build implementation:**
- `const DOCKERFILE: &str = include_str!("../Dockerfile");`
- `const ENTRYPOINT: &str = include_str!("../entrypoint.sh");`
- `cmd_build()`: write both to a `tempfile::TempDir`, run `docker build -t claudine:latest <tmpdir>`
- `check_docker()`: verify Docker is on PATH (`which::which("docker")`) and daemon is running (`docker info`)

**main.rs — routing:**
- Match on clap subcommand enum
- `Build` → `docker::cmd_build()`
- All others → `anyhow::bail!("not implemented yet")`

**Test:**
```bash
# Build the Rust binary
cargo build

# Verify help output shows all subcommands
cargo run -- --help
# expect: init, run, shell, destroy, build, list, completions

# Verify build subcommand help
cargo run -- build --help

# Verify Docker detection error (stop Docker first if you want to test this)
# cargo run -- build
# expect: clear error if Docker not running

# Build the claudine image via the CLI
cargo run -- build
# expect: Docker build output, image tagged claudine:latest

# Verify image was built
docker image inspect claudine:latest --format '{{.Id}}'
# expect: image ID

# Verify embedded files match source files
# (the build should produce the same image as step 1's manual docker build)
docker run --rm claudine:latest claude --version
```

---

## Step 3: Config Module

Implement config loading, saving, and defaults.

**Files:**
- `src/config.rs` — structs, load/save, defaults, config dir creation

**Implementation:**
- `GlobalConfig` and `ProjectConfig` structs with serde derive
- `config_dir()` → `$CONFIG_DIR/` (use `dirs` crate or `$HOME`)
- `load_global()` → deserialize `config.toml`, create with defaults if missing
- `load_project(name)` → deserialize `projects/<name>/config.toml`, error if missing
- `save_project(name, config)` → serialize and write
- `list_projects()` → read `projects/` subdirectories
- Resolve image name: project config overrides global config

**Add to Cargo.toml:**
```toml
dirs = "6"
```

**Test:**
```bash
# Verify config dir creation
cargo run -- build
ls $CONFIG_DIR/config.toml
# expect: file exists with default [image] section

# Verify default global config content
cat $CONFIG_DIR/config.toml
# expect:
# [image]
# name = "claudine:latest"

# Manually create a project config and verify it loads
mkdir -p $CONFIG_DIR/projects/testproject
cat > $CONFIG_DIR/projects/testproject/config.toml << 'EOF'
[project]
repo_url = "git@github.com:test/repo.git"
branch = "main"
EOF

# Verify list shows the project
cargo run -- list
# expect: "testproject" in output

# Cleanup
rm -rf $CONFIG_DIR/projects/testproject
```

---

## Step 4: Project Helpers

Implement project name validation and Docker volume/container helpers.

**Files:**
- `src/project.rs` — validation, volume ops, container status

**Implementation:**
- `validate_name(name)` → regex `^[a-zA-Z0-9][a-zA-Z0-9_-]*$`, return `Result`
- `volume_name(project)` → `format!("claudine_{project}")`
- `container_name(project)` → `format!("claudine_{project}")`
- `volume_exists(project)` → `docker volume inspect` exit code
- `container_running(project)` → `docker ps --filter name=... --format {{.Names}}`
- `create_volume(project)` → `docker volume create`
- `remove_volume(project)` → `docker volume rm`

**Test:**
```bash
# Verify name validation (add a unit test in project.rs)
cargo test
# expect: valid names pass, invalid names ("my project", "../escape", "") fail

# Verify volume operations manually via the module
# (these are tested end-to-end in step 5, but unit tests cover the logic)
cargo test
```

**Unit tests to include in project.rs:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_project_names() {
        assert!(validate_name("myproject").is_ok());
        assert!(validate_name("my-project").is_ok());
        assert!(validate_name("my_project_2").is_ok());
    }

    #[test]
    fn invalid_project_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name("my project").is_err());
        assert!(validate_name("../escape").is_err());
        assert!(validate_name(".hidden").is_err());
    }
}
```

---

## Step 5: `init` Command

Implement interactive project initialization.

**Files:**
- `src/init.rs` — interactive prompts, volume creation, git clone

**Implementation:**
- `cmd_init(project)`:
  1. `validate_name(project)`
  2. If `volume_exists(project)`, prompt: "Volume exists. Re-init? (y/N)"
  3. `dialoguer::Input` for repo URL (required)
  4. `dialoguer::Input` for branch (optional, default empty)
  5. `create_volume(project)`
  6. `save_project(project, config)`
  7. Build docker args for clone container:
     - `--rm`
     - `-v claudine_<project>:/workspace`
     - `-v ~/.gitconfig:/host-config/gitconfig:ro` (if exists)
     - `-v ~/.ssh:/host-config/ssh:ro` (if exists)
     - Image name from config
     - Command: `git clone [--branch <branch>] <url> /workspace/project`
  8. Run clone via `Command::new("docker")`, stream output
  9. Print success message

**Test:**
```bash
# Init a project with a public repo (no SSH needed)
cargo run -- init testproject
# prompt: enter https://github.com/jstockdi/rodney.git
# prompt: branch (leave empty)
# expect: volume created, clone output, success message

# Verify volume exists
docker volume inspect claudine_testproject
# expect: volume metadata

# Verify config was written
cat $CONFIG_DIR/projects/testproject/config.toml
# expect: repo_url and branch

# Verify clone contents by running a temp container
docker run --rm -v claudine_testproject:/workspace claudine:latest \
  bash -c 'ls /workspace/project && stat -c "%U" /workspace/project'
# expect: repo files listed, owned by "claude"

# Verify re-init prompts for confirmation
cargo run -- init testproject
# expect: "Volume exists. Re-init?" prompt

# Verify invalid name is rejected
cargo run -- init "../bad-name"
# expect: error message about invalid project name

# Verify list shows the project
cargo run -- list
# expect: testproject with repo URL
```

---

## Step 6: `run` Command

Implement the core command — launch Claude Code in a container.

**Files:**
- `src/docker.rs` — add `build_run_args()` and `cmd_run()`

**Implementation:**
- `cmd_run(project, extra_args)`:
  1. `validate_name(project)`
  2. If `!volume_exists(project)`, error with init suggestion
  3. If `container_running(project)`, error with shell suggestion
  4. `load_project(project)` to get config
  5. Resolve image name (project override or global default)
  6. `build_run_args()` assembles:
     - `--rm`, `--name claudine_<project>`
     - `-v claudine_<project>:/workspace`
     - `-v /var/run/docker.sock:/var/run/docker.sock`
     - `-v ~/.gitconfig:/host-config/gitconfig:ro` (if path exists)
     - `-v ~/.ssh:/host-config/ssh:ro` (if path exists)
     - `-v ~/.claude:/host-config/claude-credentials:ro` (if path exists)
     - `-w /workspace/project`
     - `-e HOME=/workspace/home`
     - `--shm-size=256m`
     - `-e ANTHROPIC_API_KEY` (if env var is set)
     - `-it` (only if `stdin().is_terminal()`)
  7. Command: `claude <extra_args>`
  8. Use `std::os::unix::process::CommandExt::exec()` to replace the process

**Test:**
```bash
# Requires testproject from step 5

# Verify uninit project gives clear error
cargo run -- run nonexistent
# expect: "not initialized" error with init suggestion

# Run Claude in the container
cargo run -- run testproject
# expect: Claude Code starts, working directory is the cloned repo
# inside claude: run "pwd" to verify /workspace/project
# inside claude: run "whoami" to verify "claude"
# inside claude: run "docker ps" to verify DooD works
# inside claude: run "git status" to verify repo is functional
# inside claude: run "git config user.name" to verify gitconfig
# exit claude

# Verify container was cleaned up (--rm)
docker ps -a --filter name=claudine_testproject
# expect: no containers listed

# Verify extra args pass through
cargo run -- run testproject --help
# expect: claude --help output
```

---

## Step 7: `shell` Command

Implement shell access with container reuse via `docker exec`.

**Files:**
- `src/docker.rs` — add `cmd_shell()`

**Implementation:**
- `cmd_shell(project)`:
  1. `validate_name(project)`
  2. If `!volume_exists(project)`, error with init suggestion
  3. If `container_running(project)`:
     - `docker exec -it claudine_<project> bash`
     - TTY detection applies to exec too
  4. If not running:
     - Same `build_run_args()` as `cmd_run` but no command (entrypoint defaults to bash)
     - `exec()` to replace process

**Test:**
```bash
# Shell into a stopped project (fresh container)
cargo run -- shell testproject
# expect: bash prompt inside container
# verify: pwd = /workspace/project, whoami = claude
# exit

# Test container reuse:
# Terminal 1: start claude
cargo run -- run testproject

# Terminal 2: shell into running container
cargo run -- shell testproject
# expect: bash prompt in the SAME container (docker exec)
# verify: can see same /workspace/project
# verify: docker ps shows only ONE claudine_testproject container
# exit both terminals

# Verify run refuses when container is already active:
# Terminal 1: start claude
cargo run -- run testproject
# Terminal 2: try run again
cargo run -- run testproject
# expect: error telling you to use "claudine shell" instead
```

---

## Step 8: `destroy` + `list` Commands

Implement cleanup and project listing.

**Files:**
- `src/docker.rs` — add `cmd_destroy()`
- `src/main.rs` — add `cmd_list()` (or a new `src/list.rs`)

**Implementation:**

**cmd_destroy(project):**
1. `validate_name(project)`
2. `dialoguer::Confirm` — "This will delete all data for '<project>'. Continue?"
3. If `container_running(project)`, stop it: `docker stop claudine_<project>`
4. `remove_volume(project)`
5. Remove config dir: `$CONFIG_DIR/projects/<project>/`
6. Print confirmation

**cmd_list():**
1. Read `$CONFIG_DIR/projects/` subdirectories
2. For each project:
   - Load config (repo_url)
   - Check if volume exists
   - Check if container is running
3. Print table: `NAME | REPO | STATUS`
   - Status: `running`, `stopped`, `no volume` (config exists but volume was removed)

**Test:**
```bash
# Verify list output
cargo run -- list
# expect: testproject listed with repo URL and "stopped" status

# Start a container and check list
# Terminal 1:
cargo run -- run testproject
# Terminal 2:
cargo run -- list
# expect: testproject shows "running" status

# Stop the container (exit claude in terminal 1), then destroy
cargo run -- destroy testproject
# expect: confirmation prompt
# answer: y
# expect: volume removed, config removed, success message

# Verify cleanup
docker volume inspect claudine_testproject 2>&1
# expect: error (volume gone)

ls $CONFIG_DIR/projects/testproject 2>&1
# expect: error (dir gone)

cargo run -- list
# expect: no projects (or empty list)
```

---

## Step 9: `completions` Command + Polish

Implement shell completions and final polish.

**Files:**
- `src/cli.rs` — add completions generation
- `src/main.rs` — wire up completions command

**Implementation:**
- Use `clap_complete` to generate completions for bash, zsh, fish
- `cmd_completions(shell)`: print completions to stdout
- Add `--version` flag to the top-level CLI
- Review all error messages for clarity and consistency
- Ensure all commands print to stderr for status and stdout for data (completions, list output)

**Add to Cargo.toml:**
```toml
clap_complete = "4"
```

**Test:**
```bash
# Generate zsh completions
cargo run -- completions zsh > /tmp/_claudine
# expect: valid zsh completion script

# Source and test (zsh)
source /tmp/_claudine
claudine <TAB>
# expect: init, run, shell, destroy, build, list, completions

# Verify --version
cargo run -- --version
# expect: claudine <version>

# Full end-to-end smoke test
cargo run -- build
cargo run -- init smoketest
# enter a public repo URL
cargo run -- list
cargo run -- run smoketest
# verify claude works, exit
cargo run -- shell smoketest
# verify bash works, exit
cargo run -- destroy smoketest
# confirm: y
cargo run -- list
# expect: clean
```

---

## Step 10: Install + Release

Build release binary and set up installation path.

**Files:**
- Update `README.md` (if requested)

**Implementation:**
- `cargo build --release`
- Copy `target/release/claudine` to `~/.local/bin/claudine` (or `~/.cargo/bin/` via `cargo install --path .`)
- Verify the installed binary works outside the source tree

**Test:**
```bash
# Build release binary
cargo build --release

# Install
cargo install --path .

# Verify it works from anywhere (not in the source dir)
cd /tmp
claudine --version
claudine build
claudine init release-test
# enter a public repo
claudine run release-test
# verify claude starts
# exit
claudine destroy release-test

# Verify binary is self-contained (no source dir dependency)
ls -la $(which claudine)
# expect: single binary in ~/.cargo/bin/
```
