// cumulus — a TUI for AWS services.
//
// Usage:
//
//	cumulus
//
// Credentials are resolved via the standard AWS SDK credential chain:
// environment variables, shared credentials file, and SSO token cache.
// No credentials are ever stored or logged by cumulus itself.
package main

import (
	"context"
	"fmt"
	"os"

	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/config"
	"github.com/sumsar01/cumulus/internal/services"
	"github.com/sumsar01/cumulus/internal/services/dynamodb"
	"github.com/sumsar01/cumulus/internal/ui"

	tea "github.com/charmbracelet/bubbletea"
)

func main() {
	// ── Load application config ───────────────────────────────────────────────
	appCfg, err := config.Load()
	if err != nil {
		fmt.Fprintf(os.Stderr, "cumulus: config error: %v\n", err)
		os.Exit(1)
	}

	// ── Load AWS credentials ──────────────────────────────────────────────────
	// This uses the standard SDK credential chain — SSO, env vars, profile.
	// cumulus does not handle, store, or log credentials itself.
	awsCfg, err := awspkg.LoadDefault(context.Background())
	if err != nil {
		fmt.Fprintf(os.Stderr, "cumulus: AWS config error: %v\n\nMake sure you are logged in with aws sso login.\n", err)
		os.Exit(1)
	}

	region, _ := awspkg.RegionFromConfig(awsCfg)

	// Attempt to resolve the active profile name for display in the status bar.
	profile := activeProfileName()

	// ── Register services ─────────────────────────────────────────────────────
	// To add a new service, Register it here. No other files need to change.
	services.Register(dynamodb.Svc{AppCfg: appCfg})

	// ── Build root model ──────────────────────────────────────────────────────
	navigator := ui.NewNavigator(awsCfg)
	app := ui.New(navigator, awsCfg, appCfg, profile, region)

	// ── Run ───────────────────────────────────────────────────────────────────
	p := tea.NewProgram(
		app,
		tea.WithAltScreen(),
		tea.WithMouseCellMotion(),
	)
	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "cumulus: fatal: %v\n", err)
		os.Exit(1)
	}
}

// activeProfileName returns the name of the currently active AWS profile.
// It falls back to "default" when no profile is explicitly set.
func activeProfileName() string {
	// AWS_PROFILE takes precedence, then AWS_DEFAULT_PROFILE.
	for _, env := range []string{"AWS_PROFILE", "AWS_DEFAULT_PROFILE"} {
		if v := os.Getenv(env); v != "" {
			return v
		}
	}
	return "default"
}
