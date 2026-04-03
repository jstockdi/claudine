# Claudine

Run [Claude Code](https://docs.anthropic.com/en/docs/claude-code) inside isolated Docker containers with per-project persistent volumes, multi-repo support, and automatic host config forwarding.

## Quick Start

```bash
# Install
cargo install --path .

# Build the Docker image
claudine build

# Initialize a project
claudine init myproject

# Run Claude Code
claudine run myproject my-repo
```

## Features

- **Isolated environments** — each project runs in its own Docker container with a persistent volume
- **Multi-repo projects** — init multiple repos into one project, switch between them
- **SSH key isolation** — only the key you choose is available inside the container
- **Container reuse** — multiple terminals share one container via `docker exec`
- **Host file sharing** — `~/claudine-share/<project>/` is mounted at `/share` inside the container
- **Docker-outside-of-Docker** — Claude can run Docker commands that execute on the host daemon
- **Security hooks** — [ward](https://github.com/jstockdi/ward) PII/secrets scanning built into Claude Code hooks

## Commands

```
claudine init <project>                  Create volume, clone repo(s)
claudine run <project> [repo] [-- ...]   Run Claude Code
claudine shell <project> [repo]          Open bash shell
claudine destroy <project>               Remove volume + config
claudine repo add <project> <url>        Add a repo to a project
claudine repo remove <project> <dir>     Remove a repo
claudine repo list <project>             List repos in a project
claudine build                           Build/rebuild the Docker image
claudine list                            List all projects
claudine completions <shell>             Generate shell completions
```

## Container Layout

```
/project/              Volume mount (persistent)
├── home/              $HOME — configs, credentials, SSH key
├── <repo1>/           First repository
├── <repo2>/           Second repository
└── ...

/share/                Bind mount to ~/claudine-share/<project>/
```

## Documentation

- [Architecture](docs/architecture.md) — design, data flows, and decisions
- [Implementation Plan](docs/implementation.md) — step-by-step build plan

### Issues

- [001 — Build Notes](docs/issues/001-build-notes.md) — resolved issues from initial build
- [002 — Plugin Support](docs/issues/002-plugin-support.md) — proposed plugin system
- [003 — Config Dir Platform Difference](docs/issues/003-config-dir-platform-difference.md) — resolved macOS/Linux path difference
- [004 — Multi-Repo Projects](docs/issues/004-multi-repo-projects.md) — implemented multi-repo support

## Requirements

- Docker
- Rust (for building from source)

## License

[MIT](LICENSE)
