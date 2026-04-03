#!/usr/bin/env bash
set -euo pipefail

# Ensure home directory exists with correct ownership
mkdir -p /project/home
chown claude:claude /project/home
chown -R claude:claude /project/home

# Copy host configs into the home directory
if [ -d /host-config ]; then

    # gitconfig
    if [ -f /host-config/gitconfig ]; then
        cp /host-config/gitconfig /project/home/.gitconfig
        chown claude:claude /project/home/.gitconfig
    fi

    # SSH key — single key file for security isolation
    if [ -f /host-config/ssh_key ]; then
        mkdir -p /project/home/.ssh
        cp /host-config/ssh_key /project/home/.ssh/id_key
        chmod 700 /project/home/.ssh
        chmod 600 /project/home/.ssh/id_key
        chown -R claude:claude /project/home/.ssh
        # Write minimal SSH config to use this key by default
        cat > /project/home/.ssh/config <<SSHEOF
Host *
    IdentityFile /project/home/.ssh/id_key
    IdentitiesOnly yes
    StrictHostKeyChecking accept-new
SSHEOF
        chmod 600 /project/home/.ssh/config
        chown claude:claude /project/home/.ssh/config
    fi

    # Claude credentials directory (~/.claude/)
    if [ -d /host-config/claude-credentials ]; then
        mkdir -p /project/home/.claude
        cp -a /host-config/claude-credentials/. /project/home/.claude/
        chown -R claude:claude /project/home/.claude
    fi

    # Claude config file (~/.claude.json) — may be separate from ~/.claude/
    if [ -f /host-config/claude-json ]; then
        cp /host-config/claude-json /project/home/.claude.json
        chown claude:claude /project/home/.claude.json
    fi
fi

# Write container-specific Claude settings (overrides host settings)
mkdir -p /project/home/.claude
cat > /project/home/.claude/settings.json <<'SETTINGS'
{
  "permissions": {
    "allow": [
      "Bash(aws:*)",
      "Bash(docker:*)",
      "Bash(find:*)",
      "Bash(git:*)",
      "Bash(gh:*)",
      "Bash(ls:*)",
      "Bash(npm:*)",
      "Bash(tail:*)",
      "Bash(wc:*)",
      "WebSearch"
    ]
  },
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "ward pii",
            "timeout": 5,
            "statusMessage": "Scanning for PII..."
          },
          {
            "type": "command",
            "command": "ward leaks",
            "timeout": 5,
            "statusMessage": "Scanning for secrets..."
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "ward pii",
            "timeout": 5,
            "statusMessage": "Scanning for PII..."
          },
          {
            "type": "command",
            "command": "ward leaks",
            "timeout": 5,
            "statusMessage": "Scanning for secrets..."
          }
        ]
      }
    ]
  }
}
SETTINGS
chown claude:claude /project/home/.claude/settings.json

# Mark all directories as safe for git
gosu claude git config --global --add safe.directory '*'

# Docker socket GID detection: allow the claude user to access the host Docker daemon
if [ -S /var/run/docker.sock ]; then
    DOCKER_SOCK_GID=$(stat -c '%g' /var/run/docker.sock)
    if ! getent group "${DOCKER_SOCK_GID}" > /dev/null 2>&1; then
        groupadd -g "${DOCKER_SOCK_GID}" dockerhost
    fi
    DOCKER_GROUP_NAME=$(getent group "${DOCKER_SOCK_GID}" | cut -d: -f1)
    usermod -aG "${DOCKER_GROUP_NAME}" claude
fi

# Drop privileges and execute the requested command (or bash if none given)
if [ $# -eq 0 ]; then
    exec gosu claude bash
else
    exec gosu claude "$@"
fi
