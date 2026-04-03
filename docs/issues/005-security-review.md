# Security Review Findings

**Date:** 2026-04-03  
**Status:** Open

## Critical

### Docker socket grants host root equivalent
The Docker socket mount (`/var/run/docker.sock`) combined with `--dangerously-skip-permissions` gives Claude Code effective root access to the host. This is by design (DooD pattern) but must be documented prominently.

**Action:** Document in README and architecture.md.

## High

### curl-pipe-bash for Claude CLI install
`curl -fsSL https://claude.ai/install.sh | bash` in the Dockerfile has no checksum verification. A compromised CDN could inject code.

**Action:** Pin version or verify checksum when the installer supports it.

## Medium — Fixed

### Unvalidated repo `dir` field enables path traversal
The `dir` field in `RepoConfig` was not validated. A `dir` of `../etc` or `home/.ssh` could escape `/project/` or overwrite sensitive files. Used in `git clone` target, `rm -rf` target, and Docker `-w` workdir.

**Fix:** Added `validate_dir()` function — same rules as project names. Applied in init, repo add, and `repo_dir_from_url` output.

### Git option injection via repo URLs
A URL starting with `-` (e.g., `--upload-pack=evil`) could be interpreted as a git option.

**Fix:** Added `--` before positional args in git clone commands. Reject URLs starting with `-`.

### Unpinned ward repo clone
`git clone https://github.com/jstockdi/ward.git` clones HEAD without pinning.

**Action:** Pin to a specific commit or tag in the Dockerfile.

### `safe.directory '*'` disables git ownership checks
Wildcard safe directory bypasses git's ownership security for all directories.

**Action:** Consider scoping to specific repo paths. Current trade-off is acceptable for container isolation.

## Low

- SSH key persists in volume — documented risk, acceptable for dev tooling
- ANTHROPIC_API_KEY visible via `docker inspect` — standard Docker behavior
- SSH config uses `StrictHostKeyChecking accept-new` — acceptable for automation
- Config files created with default permissions — consider 700/600
- No length limit on project names — add max 64 chars
- Base image not pinned to digest — pin for production
