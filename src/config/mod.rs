//! Configuration loading and saving for cumulus.
//!
//! Reads/writes `~/.config/cumulus/config.toml` (honouring `XDG_CONFIG_HOME`).
//! Enforces an editor allowlist so the editor binary is never passed through a
//! shell — no injection surface.

use std::{collections::HashSet, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Editor allowlist ──────────────────────────────────────────────────────────

/// The complete set of editor binaries that cumulus will exec directly.
/// Values are matched exactly — no shell expansion is performed.
pub const ALLOWED_EDITORS: &[&str] = &["nvim", "vim", "vi", "nano", "emacs", "hx", "micro"];

// ── Config struct ─────────────────────────────────────────────────────────────

/// User-facing configuration for cumulus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Binary name (not a path, not a shell expression) of the editor to open
    /// when adding/editing DynamoDB items. Must be one of [`ALLOWED_EDITORS`].
    #[serde(default = "default_editor")]
    pub editor: String,

    /// Most recently used AWS profile. Written automatically when the user
    /// switches so that the same profile is restored on next launch.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_profile: String,

    /// Name of the colour theme. Must match a built-in theme name (e.g.
    /// `"tokyo-night"`, `"catppuccin-mocha"`). Defaults to `"tokyo-night"`.
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_editor() -> String {
    "nvim".to_string()
}

fn default_theme() -> String {
    "tokyo-night".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            last_profile: String::new(),
            theme: default_theme(),
        }
    }
}

impl Config {
    /// Validate that the editor is in the allowlist.
    pub fn validate(&self) -> Result<()> {
        let allowed: HashSet<&str> = ALLOWED_EDITORS.iter().copied().collect();
        if !allowed.contains(self.editor.as_str()) {
            anyhow::bail!(
                "editor {:?} is not in the allowlist; permitted values: {:?}",
                self.editor,
                ALLOWED_EDITORS,
            );
        }
        Ok(())
    }
}

// ── Load / Save ───────────────────────────────────────────────────────────────

/// Read the config file from the standard XDG location.
///
/// If the file does not exist, `Config::default()` is returned without error.
/// Any other read or parse error is returned to the caller.
pub fn load() -> Result<Config> {
    let path = config_path()?;

    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(e) => {
            return Err(e).with_context(|| format!("reading config file {}", path.display()));
        }
    };

    let mut cfg: Config =
        toml::from_str(&data).with_context(|| format!("parsing config file {}", path.display()))?;

    // Fall back to default editor if the field was blank in the file.
    if cfg.editor.is_empty() {
        cfg.editor = default_editor();
    }
    if cfg.theme.is_empty() {
        cfg.theme = default_theme();
    }

    cfg.validate()?;
    Ok(cfg)
}

/// Write the config to disk at the standard XDG location, creating the
/// directory if needed.
pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("creating config directory {}", dir.display()))?;
    }
    let toml_str = toml::to_string_pretty(cfg).context("serialising config")?;
    fs::write(&path, toml_str)
        .with_context(|| format!("writing config file {}", path.display()))?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the absolute path to the config file, honouring `XDG_CONFIG_HOME`.
fn config_path() -> Result<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        dirs::home_dir()
            .context("resolving home directory")?
            .join(".config")
    };
    Ok(base.join("cumulus").join("config.toml"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn invalid_editor_rejected() {
        let cfg = Config {
            editor: "bash".to_string(),
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn allowed_editors_all_pass() {
        for editor in ALLOWED_EDITORS {
            let cfg = Config {
                editor: editor.to_string(),
                ..Config::default()
            };
            cfg.validate().unwrap_or_else(|e| panic!("{editor}: {e}"));
        }
    }

    #[test]
    fn round_trip_toml() {
        let cfg = Config {
            editor: "vim".to_string(),
            last_profile: "myprofile".to_string(),
            theme: "gruvbox".to_string(),
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let cfg2: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg.editor, cfg2.editor);
        assert_eq!(cfg.last_profile, cfg2.last_profile);
        assert_eq!(cfg.theme, cfg2.theme);
    }
}
