// Package aws provides a shared AWS configuration and session management layer.
// It wraps the AWS SDK v2 credential chain so that all services in cumulus share
// a single, consistently-initialised config. Credential values are never logged
// or stored beyond the in-memory aws.Config struct managed by the SDK.
package aws

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/feature/ec2/imds"
	tea "github.com/charmbracelet/bubbletea"
)

// ProfileChangedMsg is broadcast on the bubbletea message bus whenever the
// active AWS profile changes. All views should handle this message by
// discarding their cached data and re-initialising their AWS clients.
type ProfileChangedMsg struct {
	Cfg     aws.Config
	Profile string
	Region  string
}

// ErrMsg carries an error back to the bubbletea update loop.
type ErrMsg struct{ Err error }

func (e ErrMsg) Error() string { return e.Err.Error() }

// LoadDefault initialises an aws.Config using the standard credential chain:
// environment variables → shared credentials file → SSO token cache.
// EC2 IMDS is explicitly disabled since cumulus runs as a local developer tool
// and the IMDS endpoint is unreachable outside EC2 (causing slow retries).
// No credentials are read or stored by cumulus itself.
func LoadDefault(ctx context.Context) (aws.Config, error) {
	cfg, err := config.LoadDefaultConfig(ctx,
		config.WithEC2IMDSClientEnableState(imds.ClientDisabled),
	)
	if err != nil {
		return aws.Config{}, fmt.Errorf("loading AWS config: %w", err)
	}
	return cfg, nil
}

// LoadProfile initialises an aws.Config for the named profile.
// EC2 IMDS is disabled for the same reason as LoadDefault.
func LoadProfile(ctx context.Context, profile string) (aws.Config, error) {
	cfg, err := config.LoadDefaultConfig(ctx,
		config.WithSharedConfigProfile(profile),
		config.WithEC2IMDSClientEnableState(imds.ClientDisabled),
	)
	if err != nil {
		return aws.Config{}, fmt.Errorf("loading AWS config for profile %q: %w", profile, err)
	}
	return cfg, nil
}

// LoadProfileCmd returns a bubbletea Cmd that loads an AWS profile
// asynchronously and emits a ProfileChangedMsg (or ErrMsg) on completion.
func LoadProfileCmd(profile string) tea.Cmd {
	return func() tea.Msg {
		cfg, err := LoadProfile(context.Background(), profile)
		if err != nil {
			return ErrMsg{Err: err}
		}
		region, _ := RegionFromConfig(cfg)
		return ProfileChangedMsg{
			Cfg:     cfg,
			Profile: profile,
			Region:  region,
		}
	}
}

// RegionFromConfig returns the region that is configured in the given
// aws.Config, falling back to the AWS_REGION / AWS_DEFAULT_REGION env vars,
// then to "us-east-1" if nothing is set.
func RegionFromConfig(cfg aws.Config) (string, bool) {
	if cfg.Region != "" {
		return cfg.Region, true
	}
	for _, env := range []string{"AWS_REGION", "AWS_DEFAULT_REGION"} {
		if r := os.Getenv(env); r != "" {
			return r, true
		}
	}
	return "us-east-1", false
}

// ListProfiles parses ~/.aws/config and returns all profile names it finds.
// The "default" profile is always included first when present.
// Parsing is done with stdlib string splitting – we deliberately avoid shelling
// out or using any external ini library to minimise the attack surface.
func ListProfiles() ([]string, error) {
	path, err := awsConfigPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path) // #nosec G304 – path derived from $HOME, not user input
	if os.IsNotExist(err) {
		return []string{"default"}, nil
	}
	if err != nil {
		return nil, fmt.Errorf("reading AWS config %s: %w", path, err)
	}

	var profiles []string
	seen := map[string]struct{}{}

	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		// Section headers: [default], [profile foo], [sso-session bar]
		if !strings.HasPrefix(line, "[") || !strings.HasSuffix(line, "]") {
			continue
		}
		section := line[1 : len(line)-1]
		var name string
		switch {
		case section == "default":
			name = "default"
		case strings.HasPrefix(section, "profile "):
			name = strings.TrimPrefix(section, "profile ")
		default:
			// sso-session, services, etc. – skip
			continue
		}
		name = strings.TrimSpace(name)
		if name == "" {
			continue
		}
		if _, ok := seen[name]; ok {
			continue
		}
		seen[name] = struct{}{}
		profiles = append(profiles, name)
	}

	if len(profiles) == 0 {
		return []string{"default"}, nil
	}

	// Ensure "default" is first when present.
	for i, p := range profiles {
		if p == "default" && i != 0 {
			profiles = append([]string{"default"}, append(profiles[:i], profiles[i+1:]...)...)
			break
		}
	}

	return profiles, nil
}

// awsConfigPath returns the path to the shared AWS config file, honouring the
// AWS_CONFIG_FILE environment variable when set.
func awsConfigPath() (string, error) {
	if p := os.Getenv("AWS_CONFIG_FILE"); p != "" {
		return p, nil
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("resolving home directory: %w", err)
	}
	return filepath.Join(home, ".aws", "config"), nil
}
