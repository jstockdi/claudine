Based on the pre-scan data below, determine what's needed to initialize this as a claudine project.

Do NOT use any tools. All the information you need is in the prescan data.

## Available claudine plugins (use ONLY these exact names):
- `node-20` (Node 20.x or unspecified), `node-22` (Node 22.x), `node-24` (Node 24.x)
- `python-venv`, `rust` (includes just), `go`
- `gh` (GitHub CLI), `glab` (GitLab CLI)
- `aws` (AWS CLI v2), `heroku` (requires a node plugin), `lin` (Linear CLI), `rodney` (Chrome automation)

## Rules
- `git@` remotes mean SSH is required
- Dependencies must come before dependents in the plugins list (e.g. `node-20` before `heroku`)
- If tech is detected that has no matching plugin, add it to `suggested_plugins`
- Only include plugins clearly needed by the project

## Output

Write 2-3 lines summarizing the project, then output this JSON as the LAST fenced code block:

```json
{
  "repos": [
    {"url": "remote-url", "dir": "directory-name", "branch": "branch-or-null"}
  ],
  "plugins": ["plugin-name"],
  "suggested_plugins": [
    {"name": "proposed-name", "reason": "why"}
  ],
  "ssh_key_needed": true
}
```
