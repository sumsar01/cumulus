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

	"github.com/aws/aws-sdk-go-v2/aws"
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

	// ── Apply theme ───────────────────────────────────────────────────────────
	// Must happen before any UI is constructed so all style vars are correct.
	ui.SetThemeByName(appCfg.Theme)

	// ── Load AWS credentials ──────────────────────────────────────────────────
	// Determine which profile to use, in priority order:
	//   1. AWS_PROFILE / AWS_DEFAULT_PROFILE env var (explicit override)
	//   2. last_profile saved in cumulus config (restored from previous session)
	//   3. SDK default credential chain
	profile := activeProfileName()
	envProfileSet := os.Getenv("AWS_PROFILE") != "" || os.Getenv("AWS_DEFAULT_PROFILE") != ""

	var awsCfg aws.Config
	if !envProfileSet && appCfg.LastProfile != "" {
		// Restore the profile from the last session.
		profile = appCfg.LastProfile
		cfg, err := awspkg.LoadProfile(context.Background(), profile)
		if err != nil {
			// Last profile may no longer be valid — fall back to default.
			cfg, err = awspkg.LoadDefault(context.Background())
			if err != nil {
				fmt.Fprintf(os.Stderr, "cumulus: AWS config error: %v\n\nMake sure you are logged in with aws sso login.\n", err)
				os.Exit(1)
			}
			profile = activeProfileName()
		}
		awsCfg = cfg
	} else {
		cfg, err := awspkg.LoadDefault(context.Background())
		if err != nil {
			fmt.Fprintf(os.Stderr, "cumulus: AWS config error: %v\n\nMake sure you are logged in with aws sso login.\n", err)
			os.Exit(1)
		}
		awsCfg = cfg
	}

	region, _ := awspkg.RegionFromConfig(awsCfg)

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
