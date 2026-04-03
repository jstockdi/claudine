# 006: Plugin removal does not check reverse dependencies

## Summary

When removing a plugin via `claudine plugin remove`, the system does not verify
whether other installed plugins depend on the one being removed. For example,
if a project has `node-20` and `heroku` installed, removing `node-20` will
succeed without warning, even though `heroku` requires a Node.js plugin.

## Impact

The project image will still build and function correctly after removal because
Docker layers are independent and heroku bundles its own node runtime internally.
However, the dependency metadata becomes inconsistent: `heroku` is installed
without any of its declared prerequisites.

## Possible resolutions

1. **Warn on removal**: Print a warning that other plugins depend on the one
   being removed, but allow it to proceed.
2. **Block removal**: Refuse to remove a plugin that is a dependency of another
   installed plugin, requiring the dependent to be removed first.
3. **No action needed**: Since the `requires` check is an install-time gate and
   the actual Docker images work independently, the current behavior may be
   acceptable for a v1 implementation.

## Status

Deferred. Current behavior allows removal without checks, which matches the
task specification's test expectations.
