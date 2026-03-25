//! AWS configuration and session management for cumulus.
//!
//! Wraps the AWS SDK v2 credential chain so that all services share a single,
//! consistently-initialised config. Credential values are never logged or
//! stored beyond the in-memory `SdkConfig` managed by the SDK.
//!
//! EC2 IMDS is effectively disabled by setting an explicit region (so IMDS
//! region detection is bypassed) and a short connect timeout so that if IMDS
//! is probed for credentials it fails quickly instead of blocking.

use std::{env, fs, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use aws_config::{meta::region::RegionProviderChain, timeout::TimeoutConfig, BehaviorVersion};
use aws_types::region::Region;
use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

// ── IMDS workaround ───────────────────────────────────────────────────────────

/// Build a TimeoutConfig that makes IMDS probe fail quickly.
/// cumulus is a local developer tool — IMDS is unreachable outside EC2.
fn fast_timeout() -> TimeoutConfig {
    TimeoutConfig::builder()
        .connect_timeout(Duration::from_millis(200))
        .build()
}

// ── Config loading ────────────────────────────────────────────────────────────

/// Initialise an `SdkConfig` using the standard credential chain
/// (env vars → shared credentials file → SSO token cache).
///
/// An explicit region is injected from the environment (falling back to
/// `"us-east-1"`) so that IMDS region detection is bypassed.
pub async fn load_default() -> Result<SdkConfig> {
    let region = region_from_env().unwrap_or_else(|| "us-east-1".to_string());
    let cfg = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .timeout_config(fast_timeout())
        .load()
        .await;
    Ok(cfg)
}

/// Initialise an `SdkConfig` for the named profile.
pub async fn load_profile(profile: &str) -> Result<SdkConfig> {
    let fallback_region = region_from_env().unwrap_or_else(|| "us-east-1".to_string());
    let cfg = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(profile)
        .region(
            RegionProviderChain::default_provider()
                .or_else(Region::new(fallback_region)),
        )
        .timeout_config(fast_timeout())
        .load()
        .await;
    Ok(cfg)
}

/// Spawn a tokio task that loads an AWS profile and sends
/// `Action::ProfileChanged` (or `Action::AwsError`) on completion.
pub fn spawn_load_profile(profile: String, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        match load_profile(&profile).await {
            Ok(cfg) => {
                let region = region_from_sdk(&cfg);
                let _ = tx.send(Action::ProfileChanged {
                    cfg,
                    profile,
                    region,
                });
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(e.to_string()));
            }
        }
    });
}

/// Spawn a tokio task that loads a new config with a different region and sends
/// `Action::RegionChanged` on completion.
pub fn spawn_switch_region(current_profile: String, region: String, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let mut builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.clone()))
            .timeout_config(fast_timeout());

        if !current_profile.is_empty() && current_profile != "default" {
            builder = builder.profile_name(current_profile);
        }

        let new_cfg = builder.load().await;
        let _ = tx.send(Action::RegionChanged {
            cfg: new_cfg,
            region,
        });
    });
}

// ── Region helpers ────────────────────────────────────────────────────────────

/// Extract the region string from an `SdkConfig`, falling back to env vars
/// then `"us-east-1"`.
pub fn region_from_sdk(cfg: &SdkConfig) -> String {
    if let Some(r) = cfg.region() {
        return r.as_ref().to_string();
    }
    region_from_env().unwrap_or_else(|| "us-east-1".to_string())
}

/// Read region from `AWS_REGION` or `AWS_DEFAULT_REGION` environment variables.
pub fn region_from_env() -> Option<String> {
    env::var("AWS_REGION")
        .ok()
        .or_else(|| env::var("AWS_DEFAULT_REGION").ok())
        .filter(|s| !s.is_empty())
}

// ── Profile listing ───────────────────────────────────────────────────────────

/// Parse `~/.aws/config` and return all profile names.
///
/// `"default"` is always first when present. Parsing uses stdlib string
/// splitting — no external ini crate — to minimise the attack surface.
pub fn list_profiles() -> Result<Vec<String>> {
    let path = aws_config_path()?;

    let data = match fs::read_to_string(&path) {
        // #nosec: path derived from $HOME / AWS_CONFIG_FILE, not user input
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec!["default".to_string()]);
        }
        Err(e) => {
            return Err(e).with_context(|| format!("reading AWS config {}", path.display()));
        }
    };

    let mut profiles: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in data.lines() {
        let line = line.trim();
        // Section headers: [default], [profile foo], [sso-session bar]
        if !line.starts_with('[') || !line.ends_with(']') {
            continue;
        }
        let section = &line[1..line.len() - 1];
        let name = if section == "default" {
            "default".to_string()
        } else if let Some(rest) = section.strip_prefix("profile ") {
            rest.trim().to_string()
        } else {
            // sso-session, services, etc. — skip
            continue;
        };

        if name.is_empty() || seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());
        profiles.push(name);
    }

    if profiles.is_empty() {
        return Ok(vec!["default".to_string()]);
    }

    // Ensure "default" is first when present.
    if let Some(pos) = profiles.iter().position(|p| p == "default") {
        if pos != 0 {
            profiles.remove(pos);
            profiles.insert(0, "default".to_string());
        }
    }

    Ok(profiles)
}

/// Returns the path to the shared AWS config file, honouring `AWS_CONFIG_FILE`.
fn aws_config_path() -> Result<PathBuf> {
    if let Ok(p) = env::var("AWS_CONFIG_FILE") {
        return Ok(PathBuf::from(p));
    }
    let home = dirs::home_dir().context("resolving home directory")?;
    Ok(home.join(".aws").join("config"))
}

// ── Credential error detection ────────────────────────────────────────────────

/// Returns `true` if the error string looks like a credential or auth failure,
/// so the UI can show a setup hint instead of a raw SDK error.
pub fn is_credential_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("credentialsnotloaded")
        || lower.contains("no credentials")
        || lower.contains("unable to load credentials")
        || lower.contains("credentialserror")
        || lower.contains("invalidclienttokenid")
        || lower.contains("authfailure")
        || lower.contains("notauthorized")
        || lower.contains("accessdenied")
        || lower.contains("signaturedoesnotmatch")
        || lower.contains("tokenerror")
        || lower.contains("expiredtoken")
        || lower.contains("ssotoken")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_from_env_does_not_panic() {
        // Don't assert specific value — CI env may have these set.
        let _ = region_from_env();
    }

    #[test]
    fn is_credential_error_detects_common_patterns() {
        assert!(is_credential_error("NoCredentialProviders: no credentials"));
        assert!(is_credential_error("AuthFailure"));
        assert!(is_credential_error("AccessDenied"));
        assert!(!is_credential_error("ResourceNotFoundException"));
    }

    #[test]
    fn list_profiles_returns_default_when_no_file() {
        // Override AWS_CONFIG_FILE to a non-existent path so the test is
        // hermetic even on machines with ~/.aws/config.
        env::set_var("AWS_CONFIG_FILE", "/tmp/__cumulus_no_such_file__");
        let profiles = list_profiles().unwrap();
        assert_eq!(profiles, vec!["default"]);
        env::remove_var("AWS_CONFIG_FILE");
    }

    #[test]
    fn list_profiles_parses_ini() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            "[default]\nregion=us-east-1\n[profile staging]\nregion=eu-west-1\n[sso-session corp]\nurl=https://example.com"
        )
        .unwrap();
        env::set_var("AWS_CONFIG_FILE", f.path().to_str().unwrap());
        let profiles = list_profiles().unwrap();
        assert_eq!(profiles, vec!["default", "staging"]);
        env::remove_var("AWS_CONFIG_FILE");
    }
}
