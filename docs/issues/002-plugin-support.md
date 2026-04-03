# Plugin Support

**Status:** Proposed  
**Type:** Feature  

## Summary

Add a plugin system that lets users install curated tool/environment packages into claudine containers via a simple CLI command:

```
claudine plugin install frontend-design@claude-plugins-official
claudine plugin remove frontend-design
claudine plugin list
```

## Motivation

The current claudine image is intentionally generic — no project-specific tooling. Users who need additional tools today must write a custom Dockerfile extending `claudine:latest` and override `image.name` in their project config. This works but has friction:

- Requires Dockerfile knowledge
- No sharing/reuse across users or projects
- No versioning or update path
- Manual rebuild on every change

A plugin system would let the community publish reusable environment packages that anyone can install with one command.

## Proposed UX

```bash
# Install a plugin from an official registry
claudine plugin install frontend-design@claude-plugins-official

# Install a specific version
claudine plugin install frontend-design@claude-plugins-official:1.2.0

# Install from a GitHub repo
claudine plugin install github:user/my-plugin

# List installed plugins (global)
claudine plugin list

# List plugins for a specific project
claudine plugin list --project myproject

# Remove a plugin
claudine plugin remove frontend-design

# Update all plugins
claudine plugin update
```

## Design Questions

### What is a plugin?

A plugin needs to define:
- **Layer script** — commands to run during image build (apt-get install, curl, npm install -g, etc.)
- **Metadata** — name, version, description, author, dependencies on other plugins
- **Scope** — global (baked into base image) vs per-project (layered on top)

Possible plugin manifest format:
```toml
[plugin]
name = "frontend-design"
version = "1.0.0"
description = "Node.js, Figma CLI, and browser automation tools for frontend design work"
author = "claude-plugins-official"

[dependencies]
# Other plugins this one requires
plugins = ["node-20"]

[install]
# Dockerfile snippet to inject
dockerfile = """
RUN npm install -g @anthropic-ai/figma-cli lighthouse
RUN apt-get update && apt-get install -y chromium
"""
```

### Registry model

Options to consider:
1. **Git-based registry** — a GitHub repo (e.g. `claude-plugins-official`) where each plugin is a directory with a manifest. `claudine plugin install` clones/fetches and reads the manifest.
2. **OCI/container registry** — plugins as OCI artifacts, pulled from Docker Hub or GitHub Container Registry.
3. **Simple URL** — plugin manifest hosted at a known URL pattern, fetched via curl.

Git-based is simplest to start with and aligns with the existing toolchain.

### How plugins compose with images

Two approaches:

**A. Build-time layering (Dockerfile generation)**  
Plugins contribute Dockerfile snippets. `claudine build` concatenates them into a generated Dockerfile that extends `claudine:latest`. Each unique set of plugins produces a tagged image (e.g. `claudine:frontend-design-node20`).

- Pro: Standard Docker caching, reproducible
- Con: Rebuild required on plugin change, combinatorial image tags

**B. Runtime install (entrypoint hooks)**  
Plugins run install scripts at container startup.

- Pro: No rebuild, instant plugin changes
- Con: Slow startup, no caching, network dependency at runtime

**Recommendation:** Build-time layering (Option A). It fits the existing architecture where `claudine build` produces images and projects reference image names.

### Scope: global vs per-project

- **Global plugins** modify the base `claudine:latest` image — available to all projects
- **Per-project plugins** generate a project-specific image (e.g. `claudine:myproject`) configured in the project's `config.toml`

Both should be supported. Global is the default; `--project` flag scopes to a specific project.

### Config integration

```toml
# ~/.config/claudine/config.toml (global)
[image]
name = "claudine:latest"

[plugins]
installed = [
    { name = "node-20", source = "claude-plugins-official", version = "1.0.0" },
]

# ~/.config/claudine/projects/myproject/config.toml (per-project)
[plugins]
installed = [
    { name = "frontend-design", source = "claude-plugins-official", version = "1.2.0" },
]
```

## CLI Subcommand Structure

```
claudine plugin
├── install <name>[@<source>][:<version>]  [--project <name>]
├── remove <name>                          [--project <name>]
├── list                                   [--project <name>]
├── update [<name>]                        [--project <name>]
└── search <query>
```

This would be added to `cli.rs` as a nested subcommand under a new `Plugin` variant.

## Open Questions

1. **Registry hosting** — Who maintains `claude-plugins-official`? What's the review/publishing process?
2. **Security** — Plugins run arbitrary install commands. How do we trust them? Signing? Checksums?
3. **Dependency resolution** — How complex should plugin dependency handling be? Full solver or simple linear ordering?
4. **Image tagging** — How to name images produced by different plugin combinations without tag explosion?
5. **Offline support** — Should plugins be cached locally after first install?
6. **Claude Code extensions** — Should plugins also be able to install Claude Code MCP servers or CLAUDE.md snippets, not just system packages?

## Implementation Phases

1. **Phase 1** — Local plugin support: manually place plugin manifests in `~/.config/claudine/plugins/`, `claudine build` reads and layers them. No registry.
2. **Phase 2** — Git registry: `claudine plugin install` fetches from a GitHub repo.
3. **Phase 3** — Search, update, dependency resolution.
4. **Phase 4** — Per-project scoping and project-specific image generation.
