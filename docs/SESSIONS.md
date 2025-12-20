# Zoom Sessions & Persistence

## Overview
Zoom sessions allow you to save, load, and share investigation states across `pm_encoder` invocations. This is critical for complex refactoring tasks where context needs to be maintained over time.

## Quick Start

```bash
# 1. Start a named session
pm_encoder . --zoom-session create:auth-refactor

# 2. Add context (automatically saved)
pm_encoder . --zoom function=validate_token

# 3. Leave and come back later
pm_encoder . --zoom-session load:auth-refactor
```

## Commands

| Command | Description |
|---------|-------------|
| `create:name` | Creates a new session and sets it as active. |
| `load:name` | Loads an existing session from disk. |
| `list` | Lists all sessions. Active session marked with `*`. |
| `show` | Shows details of the currently active session. |
| `delete:name` | Permanently removes a session. |
| `clear` | (Planned) Remove all sessions. |

## Storage
Sessions are stored in `.pm_encoder/sessions.json` in the project root. This file is human-readable JSON.

### Schema
```json
{
  "version": "1.0",
  "sessions": {
    "auth-refactor": {
      "name": "auth-refactor",
      "created_at": "2025-12-20T10:00:00Z",
      "last_accessed": "2025-12-20T10:00:00Z",
      "description": null,
      "metadata": {},
      "active_zooms": [ ... ],
      "history": { ... }
    }
  },
  "active_session": "auth-refactor"
}
```

## Best Practices
1. **Gitignore**: Add `.pm_encoder/` to your `.gitignore` if you don't want to share local investigation states.
2. **Naming**: Use kebab-case for session names (e.g., `feature-login`, `bug-123`).
3. **Reset**: To reset your view, you can simply create a new session or delete the current one.
