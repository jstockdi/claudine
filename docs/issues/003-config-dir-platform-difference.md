# 003: Config directory path differs by platform

**Status:** Resolved

## Summary

The `dirs::config_dir()` function returns platform-specific paths:
- **macOS**: `~/Library/Application Support/claudine/`
- **Linux**: `~/.config/claudine/`

The implementation spec and test commands referenced `~/.config/claudine/` which is correct for Linux but not macOS. The code itself works correctly on both platforms since it uses `dirs::config_dir()` consistently.

## Resolution

Updated `docs/implementation.md` to use a `$CONFIG_DIR` variable with a platform note at the top. No code changes needed.
