# Agent Instructions

## Project Overview

**cumulus** — a terminal UI for AWS, built with [Ratatui](https://ratatui.rs/).
Binary: `cumulus` | Repo: `cumulus`

Currently supports DynamoDB (browse, scan/query, edit items), Lambda, CloudWatch Logs, and SQS. Extensible via a service module pattern.

---

## Build, Run, and Test Commands

```bash
# Build
cargo build

# Build release binary (output: target/release/cumulus)
cargo build --release

# Run without building
cargo run

# Verify all packages compile
cargo check

# Static analysis / lints
cargo clippy

# Run all tests
cargo test

# Run a single test (by name)
cargo test test_function_name

# Run tests with output
cargo test -- --nocapture
```

There is no Makefile, justfile, or CI pipeline. No clippy config exists yet.

---

## Code Style

### Imports

Use standard Rust `use` declarations grouped: std → external crates → internal modules, with blank lines between groups.

```rust
use std::sync::Arc;

use anyhow::Result;
use ratatui::widgets::Block;

use crate::config::Config;
use crate::services::Service;
```

### Formatting

Standard `rustfmt`. No custom formatting rules — run `cargo fmt` before committing.

### Naming

| Thing | Convention | Example |
|---|---|---|
| Types / Traits | PascalCase | `TablesView`, `ServicePlugin` |
| Async actions / events | descriptive enum variants | `Action::FetchTables`, `Event::Key` |
| Constructors | `new` or `default` | `TablesView::new()` |
| Modules | snake_case | `dynamodb`, `cwlogs` |
| Error variants | PascalCase | `AwsError`, `ConfigError` |

### Error Handling

- Use `anyhow::Result` for application-level errors; `thiserror` for library-facing error types
- Always add context: `.context("fetching tables")`  or  `anyhow::bail!("fetchTables: {err}")`
- No `unwrap()` / `expect()` in production code paths (use `?` or explicit error handling)
- Annotate intentional panics with a comment explaining the invariant

### Comments

- Doc comment on every public item: `/// Short summary.`
- Section dividers: `// ── Section name ────────────────────────────────────────`
- Inline rationale for security decisions (why IMDS is disabled, why editor is allowlisted, etc.)

---

## Architecture

### Ratatui Event Loop

The app runs an async event loop (`app.rs`). UI components are structs implementing a common draw/update interface. Navigation uses an `Action` enum dispatched through a central handler.

```
App (root)
 └─ services: [Navigator | DynamoDB | Lambda | CloudWatchLogs | SQS]
```

### Async AWS I/O

All AWS calls are async via `tokio` tasks. Results are sent back via an `mpsc` channel as `Action` variants. Never block the render loop.

```rust
tokio::spawn(async move {
    let tables = client.list_tables().send().await?;
    tx.send(Action::TablesLoaded(tables.table_names)).await?;
    Ok::<_, anyhow::Error>(())
});
```

### Service Module Pattern

Add a new AWS service by creating a module under `src/services/` and registering it in `src/services/mod.rs`. Each service owns its views and API helpers.

### Profile Switching

Profile/region changes rebuild the AWS config and reinitialise all service clients.

---

## Security Conventions

- **Editor allowlist**: only `nvim`, `vim`, `vi`, `nano`, `emacs`, `hx`, `micro` are accepted in config
- **No shell interpolation**: editor commands are spawned via `std::process::Command` with args as a slice, never through a shell
- **IMDS disabled**: AWS SDK config disables EC2 IMDS endpoint
- Document any unsafe blocks or security-sensitive decisions with an inline comment

---

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Version-controlled: Built on Dolt with cell-level merge
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs with git:

- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
