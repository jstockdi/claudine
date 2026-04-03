# Multi-Repo Project Support

**Status:** Proposed  
**Type:** Feature  

## Summary

Allow a single claudine project to contain multiple git repositories, cloned as sibling directories under `/project/`. This also flattens the container layout from `/workspace/home` + `/workspace/project` to just `/home` and `/project` at the root.

## Container Layout Change

### Current

```
/workspace/
├── home/         ← $HOME
└── project/      ← single repo root, workdir
```

### Proposed

```
/home/            ← $HOME (persistent via volume)
/project/         ← workdir
├── frontend/     ← repo 1
├── backend/      ← repo 2
└── infra/        ← repo 3
```

The Docker volume mounts to `/` with two subdirectories. Simpler paths, no `/workspace` nesting. Single-repo projects just have one entry under `/project/`.

**Breaking change:** This requires updating the Dockerfile (`WORKDIR`), entrypoint.sh (all `/workspace/` references), `build_run_args()` (`-w` and `-e HOME`), and the init clone path. Existing volumes would need migration or re-init.

## UX

### Init flow

```
$ claudine init mystack

Repository URL (leave empty to finish): git@github.com:acme/frontend.git
Directory name [frontend]:
Branch [default]:

Repository URL (leave empty to finish): git@github.com:acme/backend.git
Directory name [backend]:
Branch [default]: develop

Repository URL (leave empty to finish):

Creating volume 'claudine_mystack'...
Cloning frontend...
Cloning backend...
Project 'mystack' initialized successfully.
```

- First repo is required (can't init with zero repos)
- Directory name defaults to the repo name (derived from URL — strip `.git`, take last path segment)
- User can override the directory name to avoid collisions or for clarity
- Branch is per-repo

### Adding repos later

```
$ claudine repo add mystack git@github.com:acme/infra.git
Directory name [infra]:
Branch [default]:
Cloning infra...
Done.

$ claudine repo remove mystack infra
Remove directory 'infra' from volume? (y/N): y
Done.

$ claudine repo list mystack
NAME        REPO                                  BRANCH
frontend    git@github.com:acme/frontend.git      main
backend     git@github.com:acme/backend.git       develop
infra       git@github.com:acme/infra.git         main
```

This adds a `repo` subcommand with `add`, `remove`, and `list` sub-subcommands.

## Config Changes

### Current

```toml
[project]
repo_url = "git@github.com:acme/frontend.git"
branch = "main"
```

### Proposed

```toml
[[repos]]
url = "git@github.com:acme/frontend.git"
dir = "frontend"
branch = "main"

[[repos]]
url = "git@github.com:acme/backend.git"
dir = "backend"
branch = "develop"
```

TOML array of tables (`[[repos]]`) maps cleanly to `Vec<RepoConfig>` in Rust.

### Rust structs

```rust
#[derive(Deserialize, Serialize)]
struct ProjectConfig {
    repos: Vec<RepoConfig>,
    image: Option<ImageConfig>,
}

#[derive(Deserialize, Serialize)]
struct RepoConfig {
    url: String,
    dir: String,
    branch: Option<String>,
}
```

## Implementation Impact

### Files that change

| File | Change |
|------|--------|
| `src/cli.rs` | Add `Repo` subcommand with `Add`, `Remove`, `List` children |
| `src/config.rs` | Replace `ProjectInfo` with `Vec<RepoConfig>`, migration for old format |
| `src/init.rs` | Loop prompt for multiple repos, clone each to subdirectory |
| `src/docker.rs` | No change — working dir stays `/workspace/project` |
| `src/main.rs` | Route `Repo` subcommand |

### Clone logic

Each repo clones to `/project/<dir>/`:

```
git clone <url> /project/<dir>
```

The Docker working directory (`-w /project`) is the parent — Claude sees all repos as subdirectories and can navigate between them.

### Volume mount strategy

Single volume at `/workspace`, symlinked to clean root-level paths by the entrypoint:

```
Volume claudine_<project> → /workspace
  /workspace/home/       (persistent $HOME)
  /workspace/project/    (repos live here)

Symlinks (created by entrypoint.sh):
  /home    → /workspace/home
  /project → /workspace/project
```

Docker args:
```
-v claudine_<project>:/workspace
-e HOME=/home
-w /project
```

Users and Claude see `/home` and `/project`. The volume stays unified. Entrypoint adds two lines:
```bash
ln -sfn /workspace/home /home
ln -sfn /workspace/project /project
```

### Backwards compatibility

Need to handle existing single-repo configs. Two options:

**A. Migration on load** — if config has old `[project]` format, auto-convert to `[[repos]]` with `dir` derived from URL. Write back on next save.

**B. Support both formats** — serde can try deserializing as new format first, fall back to old. More complexity for a transitional period.

Recommendation: Option A (migrate on load). Claudine is pre-1.0, no need to carry two formats.

### Repo name derivation

Extract directory name from URL:
- `git@github.com:acme/frontend.git` → `frontend`
- `https://github.com/acme/backend.git` → `backend`
- `https://github.com/acme/my.dotted.repo.git` → `my.dotted.repo`

Strip `.git` suffix, take the last path segment.

## Open Questions

1. **Single-repo ergonomics** — Most projects are single-repo. Should the init flow still feel lightweight for the common case? Maybe only prompt for additional repos if user opts in: "Add another repository? (y/N)"
2. **Git operations across repos** — Should claudine provide any multi-repo git helpers (pull all, status all) or leave that to Claude/the user?
3. **Shared dependencies** — Some multi-repo setups have cross-repo deps (e.g., shared proto files). Should claudine support symlinks or mount overlays, or is that out of scope?
4. **Path layout** — Decided: keep `/workspace` as the volume mount, symlink `/home → /workspace/home` and `/project → /workspace/project` in the entrypoint. Clean paths for users, single volume.
