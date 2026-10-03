//! Windows Terminal recipes. Startup text is explicitly a trusted shell script, not argv.
use super::ValidationError;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalShell {
    #[default]
    PowerShell,
    CommandPrompt,
    Wsl,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerminalSettings {
    pub shell: TerminalShell,
    pub script: String,
    pub distribution: String,
    pub user: String,
    pub keep_open: bool,
    /// Empty resolves the current user's installed wt.exe alias.
    pub launcher: String,
}
impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: Default::default(),
            script: String::new(),
            distribution: String::new(),
            user: String::new(),
            keep_open: true,
            launcher: String::new(),
        }
    }
}
impl TerminalSettings {
    pub fn validate(&self, directory: &str) -> Result<(), ValidationError> {
        let name = |s: &str| {
            s.len() <= 256
                && !s.chars().any(char::is_control)
                && !s.contains([';', '"', '\\'])
                && (s.is_empty() || (!s.trim().is_empty() && !s.starts_with('-')))
        };
        if self.script.len() > 8192
            || self
                .script
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            || !name(&self.distribution)
            || !name(&self.user)
            || self.launcher.len() > 32768
            || self.launcher.chars().any(char::is_control)
            || directory.contains(';')
            || (self.shell == TerminalShell::Wsl
                && (directory.contains(['"', '\\'])
                    || (!directory.is_empty() && directory != "~" && !directory.starts_with('/'))))
        {
            return Err(ValidationError(
                "Terminal: check script (max 8KiB), distribution/user and starting directory (no semicolons; no double quotes/backslashes in WSL names/paths).",
            ));
        }
        Ok(())
    }
    pub fn title(id: uuid::Uuid) -> String {
        format!("MaXImDock Terminal {id}")
    }
    /// No user script text or `;` separators reach wt's own command parser.
    pub fn arguments(&self, id: uuid::Uuid, directory: &str) -> Vec<String> {
        let mut args = vec![
            "--window".into(),
            "new".into(),
            "new-tab".into(),
            "--title".into(),
            Self::title(id),
            "--suppressApplicationTitle".into(),
        ];
        if self.shell != TerminalShell::Wsl && !directory.is_empty() {
            args.extend(["--startingDirectory".into(), directory.into()]);
        }
        match self.shell {
            TerminalShell::PowerShell | TerminalShell::CommandPrompt => {
                args.extend(["powershell.exe".into(), "-NoLogo".into()]);
                if self.shell == TerminalShell::PowerShell && self.keep_open {
                    args.push("-NoExit".into());
                }
                let script = if self.shell == TerminalShell::CommandPrompt {
                    format!(
                        "& $env:ComSpec /d /{} '{}'",
                        if self.keep_open { "k" } else { "c" },
                        self.script
                            .replace("\r\n", "\n")
                            .replace('\r', "\n")
                            .replace('\n', " & ")
                            .replace('\'', "''")
                    )
                } else if self.script.trim().is_empty() {
                    "# Open shell".into()
                } else {
                    self.script.clone()
                };
                let bytes: Vec<_> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
                args.extend(["-EncodedCommand".into(), base64(&bytes)]);
            }
            TerminalShell::Wsl => {
                args.push("wsl.exe".into());
                for (flag, value) in [
                    ("--distribution", self.distribution.as_str()),
                    ("--user", self.user.as_str()),
                    ("--cd", directory),
                ] {
                    if !value.is_empty() {
                        args.extend([flag.into(), value.into()]);
                    }
                }
                // wt reconstructs child argv by surrounding space-containing words
                // with quotes, without escaping inner quotes. Keep this bootstrap
                // quote-free; source preserves the PTY stdin for interactive tools.
                let mut script = format!(
                    "source <(printf %s {} | base64 -d)",
                    base64(
                        self.script
                            .replace("\r\n", "\n")
                            .replace('\r', "\n")
                            .as_bytes()
                    )
                );
                if self.keep_open {
                    script.push_str("\nexec /bin/bash -li");
                }
                args.extend(["--exec".into(), "/bin/bash".into(), "-lic".into(), script]);
            }
        }
        args
    }
}
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for part in bytes.chunks(3) {
        let value = (u32::from(part[0]) << 16)
            | (u32::from(*part.get(1).unwrap_or(&0)) << 8)
            | u32::from(*part.get(2).unwrap_or(&0));
        out.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        out.push(if part.len() > 1 {
            ALPHABET[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if part.len() > 2 {
            ALPHABET[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_scripts_are_encoded_and_force_a_new_identifiable_window() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        for shell in [
            TerminalShell::PowerShell,
            TerminalShell::CommandPrompt,
            TerminalShell::Wsl,
        ] {
            let settings = TerminalSettings {
                shell,
                script: "echo '日本語'; echo \"a&b\"\npython".into(),
                ..Default::default()
            };
            let args = settings.arguments(uuid::Uuid::new_v4(), "");
            assert_eq!(&args[..3], ["--window", "new", "new-tab"]);
            assert!(args.iter().all(|a| !a.contains(';')));
            assert!(args.contains(&"--suppressApplicationTitle".into()));
            settings.validate("").unwrap();
            assert_eq!(
                toml::from_str::<TerminalSettings>(&toml::to_string(&settings).unwrap()).unwrap(),
                settings
            );
        }
        assert!(
            TerminalSettings {
                shell: TerminalShell::Wsl,
                ..Default::default()
            }
            .validate("C:\\work")
            .is_err()
        );
        assert!(
            TerminalSettings::default()
                .validate("C:\\work;bad")
                .is_err()
        );
    }

    #[test]
    fn terminal_recipes_preserve_legacy_and_round_trip_without_an_executable() {
        use crate::core::{Workspace, WorkspaceEntry, WslSettings, validate_workspaces};
        let old = WorkspaceEntry {
            executable: "C:\\app.exe".into(),
            ..Default::default()
        };
        let saved = toml::to_string(&old).unwrap();
        assert!(!saved.contains("terminal"));
        assert_eq!(toml::from_str::<WorkspaceEntry>(&saved).unwrap(), old);
        let mut workspace = Workspace::new("Terminal");
        workspace.entries.push(WorkspaceEntry {
            terminal: Some(TerminalSettings::default()),
            ..Default::default()
        });
        validate_workspaces(std::slice::from_ref(&workspace)).unwrap();
        assert_eq!(
            toml::from_str::<Workspace>(&toml::to_string(&workspace).unwrap()).unwrap(),
            workspace
        );
        workspace.entries[0].wsl = Some(WslSettings::default());
        assert!(validate_workspaces(std::slice::from_ref(&workspace)).is_err());
        workspace.entries[0].wsl = None;
        workspace.entries[0].terminal.as_mut().unwrap().script = "a".repeat(8193);
        assert!(validate_workspaces(&[workspace]).is_err());
    }

    #[test]
    fn terminal_validation_and_shell_lifetime_are_explicit() {
        for shell in [
            TerminalShell::PowerShell,
            TerminalShell::CommandPrompt,
            TerminalShell::Wsl,
        ] {
            let mut settings = TerminalSettings {
                shell,
                ..Default::default()
            };
            settings.validate("").unwrap();
            settings.script = "bad\0script".into();
            assert!(settings.validate("").is_err());
            settings.script.clear();
            settings.user = "-root".into();
            assert!(settings.validate("").is_err());
            settings.user.clear();
            let id = uuid::Uuid::new_v4();
            let open = settings.arguments(id, "");
            settings.keep_open = false;
            let close = settings.arguments(id, "");
            assert_ne!(open, close);
            if shell == TerminalShell::Wsl {
                assert!(settings.validate("/home/a\"b").is_err());
                assert!(!open.last().unwrap().contains('"'));
                assert!(open.last().unwrap().contains("exec /bin/bash -li"));
                assert!(!close.last().unwrap().contains("exec /bin/bash -li"));
                assert!(settings.validate("~").is_ok());
            }
        }
    }
}
