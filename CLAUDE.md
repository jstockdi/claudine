# claudine

CLI tool for running Claude Code in isolated Docker containers.

## Build & run

```bash
cargo build --release          # dev build
cargo install --path .         # install to ~/.cargo/bin
cargo test                     # run tests
```

After changing Rust source, reinstall to pick up changes:
```bash
cargo install --path .
```

After changing `Dockerfile`, `entrypoint.sh`, or any layer source, rebuild the base image:
```bash
claudine build
```

To rebuild a project-specific image (after layer changes):
```bash
claudine build <project>
```

## Project layout

| Path | Purpose |
|------|---------|
| `src/` | Rust source |
| `src/main.rs` | CLI entry point and command dispatch |
| `src/cli.rs` | clap argument structs |
| `src/config.rs` | Config load/save (`~/Library/Application Support/claudine/`) |
| `src/init.rs` | `init` and `repo add` logic, SSH key detection, home setup |
| `src/docker.rs` | Docker image build, container run/shell/destroy |
| `src/layer.rs` | Layer catalog and Dockerfile generation |
| `src/repo.rs` | `repo` subcommands |
| `src/migrate.rs` | Legacy volume → bind-mount migration |
| `src/devcontainer.rs` | devcontainer.json generation for Zed |
| `src/project.rs` | Volume/container name helpers |
| `Dockerfile` | Base claudine image |
| `entrypoint.sh` | Container entrypoint — drops to `claude` user via gosu |
| `setup-home.sh` | One-shot home-volume setup (SSH keys, gitconfig, Claude settings) |

## Config locations (macOS)

- Global config: `~/Library/Application Support/claudine/config.toml`
- Project configs: `~/Library/Application Support/claudine/projects/<name>/config.toml`
- Project host dirs: `~/projects/<name>/`
- Home volumes: Docker named volume `claudine_<name>_home`

## Key design notes

- `entrypoint.sh` runs as root, drops to `claude` via `gosu`, then re-asserts
  `HOME` with `env HOME=...` because gosu resets it to the passwd entry. All
  claudine `docker run` calls pass `-e HOME=/project/home`.
- `setup-home.sh` runs with `--entrypoint bash` (bypasses gosu) so it can
  chown files as root before the privilege drop.
- SSH config in the home volume uses `StrictHostKeyChecking accept-new` and
  `IdentityFile /project/home/.ssh/id_key` (absolute path, gosu-safe).

## Releasing

Before pushing a release commit:

1. Add an entry to `CHANGELOG.md` under a new `## [x.y.z] - YYYY-MM-DD` heading.
   Move relevant items from `## [Unreleased]`.
2. Bump `version` in `Cargo.toml` (patch for bugfix, minor for new feature,
   major for breaking change).
3. Run `cargo build --release` to update `Cargo.lock`.
4. Commit: `git commit -m "Release v<version>"`.
5. Tag: `git tag v<version>`.
6. Update the comparison links at the bottom of `CHANGELOG.md`.

When helping with this project: if there are meaningful unreleased changes in
`CHANGELOG.md` before a push, prompt to bump the version and tag the release.
