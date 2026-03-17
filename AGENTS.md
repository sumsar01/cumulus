# Agent Instructions

## Project Overview

**cumulus** — a terminal UI for AWS, built with [Bubble Tea](https://github.com/charmbracelet/bubbletea).
Module: `github.com/sumsar01/cumulus` | Binary: `cumulus` | Repo: `aws-tui`

Currently supports DynamoDB (browse, scan/query, edit items). Extensible via a service plugin interface.

---

## Build, Run, and Test Commands

```bash
# Build
go build -o cumulus .

# Run without building
go run .

# Verify all packages compile
go build ./...

# Static analysis
go vet ./...

# Run all tests
go test ./...

# Run a single test (by name, in a specific package)
go test -run TestFunctionName ./internal/path/to/pkg/

# Run tests with verbose output
go test -v ./...
```

There is no Makefile, justfile, or CI pipeline. No linter config (`.golangci.yml`) exists yet.

---

## Code Style

### Imports

Use `goimports` grouping: stdlib → external → internal, with blank lines between groups.

```go
import (
    "context"
    "fmt"

    "github.com/aws/aws-sdk-go-v2/aws"
    tea "github.com/charmbracelet/bubbletea"

    awspkg "github.com/sumsar01/cumulus/internal/aws"
    "github.com/sumsar01/cumulus/internal/ui"
)
```

Standard aliases: `tea` for `charmbracelet/bubbletea`, `awspkg` for `internal/aws`,
`ddbtypes` for `aws-sdk-go-v2/service/dynamodb/types`.

### Formatting

Standard `gofmt`. No custom formatting rules.

### Naming

| Thing | Convention | Example |
|---|---|---|
| Types | PascalCase | `TablesModel`, `ItemsModel` |
| Tea message types | suffix `Msg` | `tablesLoadedMsg`, `ProfileChangedMsg`, `ErrMsg` |
| Constructors | prefix `New` | `NewTablesModel`, `NewSpinner` |
| Tea command funcs | suffix `Cmd` | `fetchTablesCmd`, `putItemCmd` |
| Internal bus messages | unexported | `tablesLoadedMsg`, `editorDoneMsg` |

### Error Handling

- Always wrap with context: `fmt.Errorf("fetchTables: %w", err)`
- Propagate async errors to the Tea message bus: `awspkg.ErrMsg{Err: err}`
- Use `errors.Is(err, os.ErrNotExist)` for sentinel checks
- Annotate security-reviewed calls: `// #nosec G304` (file reads), `// #nosec G204` (exec)
- No `panic()` in production code

### Comments

- Godoc comment on every exported symbol
- Section dividers: `// ── Section name ────────────────────────────────────────`
- Inline rationale for security decisions (why IMDS is disabled, why editor is allowlisted, etc.)

### Structs and Receivers

- Models are value types; pass by value
- Pointer receivers only for mutation helpers (e.g. `reset()`, `rebuildTable()`)
- `Init()` and `View()` use value receivers; `Update()` returns `(tea.Model, tea.Cmd)`

---

## Architecture

### Bubble Tea Elm Pattern

All UI models implement `tea.Model` (`Init`, `Update`, `View`). The root `App` model maintains
a view stack (`App.stack []tea.Model`). Navigation uses `PushMsg`/`PopMsg`.

```
App (root)
 └─ stack: [Navigator | TablesModel | ItemsModel | DetailModel | ...]
```

### Async AWS I/O

All AWS calls are async via `tea.Cmd` closures that return typed `tea.Msg` values.
Never block in `Update()`.

```go
func fetchTablesCmd(ctx context.Context, client *dynamodb.Client) tea.Cmd {
    return func() tea.Msg {
        // ... AWS call ...
        return tablesLoadedMsg{tables: out.TableNames}
    }
}
```

### Service Plugin Pattern

Add a new AWS service by implementing the `Service` interface in `internal/services/`:

```go
type Service interface {
    Name() string
    ShortName() string
    Description() string
    Icon() string
    Init(cfg aws.Config) (tea.Model, tea.Cmd)
}
```

Register in `main.go` with `services.Register(...)`. No other files need changing.

### Profile Switching

Profile changes broadcast `ProfileChangedMsg` to the entire stack so all models can reinitialise
their AWS clients.

---

## Security Conventions

- **Editor allowlist**: only `nvim`, `vim`, `vi`, `nano`, `emacs`, `hx`, `micro` are accepted in config
- **No shell interpolation**: editor commands are passed as `exec.Command(binary, args...)`, never through a shell
- **IMDS disabled**: AWS SDK config sets `EC2IMDSClientEnableState: imds.ClientDisabled`
- **`// #nosec` annotations**: all `gosec` suppressions are documented with a comment explaining why it is safe

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
