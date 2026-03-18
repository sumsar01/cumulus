// Package config loads and validates cumulus configuration from
// ~/.config/cumulus/config.toml. It enforces an editor allowlist so that
// the editor binary is never passed through a shell (no injection surface).
package config

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
)

// AllowedEditors is the complete set of editor binaries that cumulus will
// exec directly. Values are matched exactly – no shell expansion is performed.
// It is exported so that editor.go can use it as a defence-in-depth check
// without duplicating the list.
var AllowedEditors = map[string]struct{}{
	"nvim":  {},
	"vim":   {},
	"vi":    {},
	"nano":  {},
	"emacs": {},
	"hx":    {},
	"micro": {},
}

// Config holds user-facing configuration for cumulus.
type Config struct {
	// Editor is the binary name (not a path, not a shell expression) of the
	// editor to open when adding or editing DynamoDB items. It must be one of
	// the values in allowedEditors.
	Editor string `toml:"editor"`

	// LastProfile is the name of the most recently used AWS profile. cumulus
	// writes this automatically when the user switches profiles so that the
	// same profile is restored on the next launch.
	LastProfile string `toml:"last_profile,omitempty"`

	// Theme is the name of the colour theme to use. Must match one of the
	// built-in theme Name values (e.g. "tokyo-night", "catppuccin-mocha").
	// Defaults to "tokyo-night" when unset.
	Theme string `toml:"theme,omitempty"`
}

// Default returns a Config populated with safe defaults.
func Default() Config {
	return Config{
		Editor: "nvim",
		Theme:  "tokyo-night",
	}
}

// Load reads the config file from the standard location. If the file does not
// exist, Default() is returned without error. Any other read or parse error is
// returned to the caller.
func Load() (Config, error) {
	path, err := configPath()
	if err != nil {
		return Default(), fmt.Errorf("resolving config path: %w", err)
	}

	cfg := Default()

	data, err := os.ReadFile(path) // #nosec G304 – path is derived from XDG_CONFIG_HOME or $HOME, not user input
	if errors.Is(err, os.ErrNotExist) {
		return cfg, nil
	}
	if err != nil {
		return cfg, fmt.Errorf("reading config file %s: %w", path, err)
	}

	if _, err := toml.Decode(string(data), &cfg); err != nil {
		return cfg, fmt.Errorf("parsing config file %s: %w", path, err)
	}

	// If the editor field was missing or blank in the file, fall back to the
	// default rather than failing validation with a confusing empty-string error.
	if cfg.Editor == "" {
		cfg.Editor = Default().Editor
	}

	if err := cfg.validate(); err != nil {
		return cfg, fmt.Errorf("invalid config: %w", err)
	}

	return cfg, nil
}

// Save writes the config to disk at the standard location, creating the
// directory if needed. Fields with zero values and omitempty tags are omitted.
func Save(cfg Config) error {
	path, err := configPath()
	if err != nil {
		return fmt.Errorf("resolving config path: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return fmt.Errorf("creating config directory: %w", err)
	}
	f, err := os.Create(path) // #nosec G304 – path derived from XDG_CONFIG_HOME or $HOME
	if err != nil {
		return fmt.Errorf("writing config file %s: %w", path, err)
	}
	defer f.Close()
	return toml.NewEncoder(f).Encode(cfg)
}

func (c *Config) validate() error {
	if _, ok := AllowedEditors[c.Editor]; !ok {
		allowed := make([]string, 0, len(AllowedEditors))
		for k := range AllowedEditors {
			allowed = append(allowed, k)
		}
		return fmt.Errorf(
			"editor %q is not in the allowlist; permitted values: %v",
			c.Editor, allowed,
		)
	}
	return nil
}

// configPath returns the absolute path to the config file, honouring
// XDG_CONFIG_HOME when set.
func configPath() (string, error) {
	base := os.Getenv("XDG_CONFIG_HOME")
	if base == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("resolving home directory: %w", err)
		}
		base = filepath.Join(home, ".config")
	}
	return filepath.Join(base, "cumulus", "config.toml"), nil
}
