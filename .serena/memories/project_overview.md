# cumulus project

## Purpose
Terminal UI (TUI) for AWS services, starting with DynamoDB. Uses Tokyo Night dark theme throughout.

## Tech Stack
- Go (module: github.com/sumsar01/cumulus)
- charmbracelet/bubbletea - TUI framework
- charmbracelet/lipgloss - styling
- charmbracelet/bubbles - UI components (table, spinner, viewport)
- charmbracelet/huh - form inputs (theme in internal/ui/huh_theme.go)
- AWS SDK Go v2

## Structure
- main.go - entry point
- internal/ui/ - shared styles, navigator, statusbar, helpers, app
- internal/services/ - service registry
- internal/services/dynamodb/ - DynamoDB plugin (tables, items, detail, etc.)
- internal/config/ - config
- internal/aws/ - AWS client helpers

## Key files
- internal/ui/styles.go - all color constants and lipgloss styles (Tokyo Night theme)
- internal/ui/navigator.go - home screen service picker
- internal/ui/statusbar.go - persistent bottom bar
- internal/ui/helpers.go - shared helpers (HorizontalSep, RenderHints, etc.)
- internal/ui/huh_theme.go - huh form theme
- internal/services/dynamodb/tables.go - DynamoDB tables list
- internal/services/dynamodb/items_model.go - DynamoDB items view
- internal/services/dynamodb/detail.go - item detail view

## Commands
- go build ./... - build
- go test ./... - test
- go vet ./... - vet
- The binary is named "cumulus"
