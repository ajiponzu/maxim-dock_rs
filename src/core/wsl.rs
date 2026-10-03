//! WSL recipe options and argv construction. Linux paths must not use Windows existence checks.
use super::ValidationError;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WslSettings {
    pub distribution: String,
    pub user: String,
    pub wait_for_exit: bool,
    pub timeout_secs: u64,
    /// Use bash's login environment, while keeping command and arguments positional.
    pub login_shell: bool,
    /// Opt in to bypassing the default Linux shell. Missing field uses normal `wsl command`.
    pub direct_exec: bool,
    pub place_window: bool,
}
impl Default for WslSettings {
    fn default() -> Self {
        Self {
            distribution: String::new(),
            user: String::new(),
            wait_for_exit: true,
            timeout_secs: 60,
            login_shell: false,
            direct_exec: false,
            place_window: false,
        }
    }
}
impl WslSettings {
    pub fn validate(&self, command: &str, directory: &str) -> Result<(), ValidationError> {
        let name = |s: &str| {
            s.len() <= 256
                && !s.chars().any(char::is_control)
                && (s.is_empty() || (!s.trim().is_empty() && !s.starts_with('-')))
        };
        if !name(&self.distribution)
            || !name(&self.user)
            || !(1..=600).contains(&self.timeout_secs)
            || command.starts_with('-')
            || command.contains('\\')
            || (!directory.is_empty() && !directory.starts_with('/') && directory != "~")
        {
            return Err(ValidationError(
                "WSL: check distribution/user, Linux command/directory and timeout (1–600 seconds).",
            ));
        }
        Ok(())
    }
    pub fn arguments(&self, command: &str, arguments: &[String], directory: &str) -> Vec<String> {
        let mut args = Vec::new();
        for (flag, value) in [
            ("--distribution", self.distribution.as_str()),
            ("--user", self.user.as_str()),
            ("--cd", directory),
        ] {
            if !value.is_empty() {
                args.extend([flag.into(), value.into()]);
            }
        }
        if self.login_shell {
            args.push("--exec".into());
            args.extend(["/bin/bash", "-lc", "exec \"$@\"", "maximdock"].map(str::to_owned));
            args.push(command.into());
            args.extend_from_slice(arguments);
        } else if self.direct_exec {
            args.extend(["--exec".into(), command.into()]);
            args.extend_from_slice(arguments);
        } else {
            // wsl's default shell receives one command line. Quote every word separately;
            // never treat the user's command/argument fields as shell program text.
            let line = std::iter::once(command)
                .chain(arguments.iter().map(String::as_str))
                .map(shell_word)
                .collect::<Vec<_>>()
                .join(" ");
            args.extend(["--".into(), line]);
        }
        args
    }
    pub fn execution_mode(&self) -> &'static str {
        if self.login_shell {
            "bash ログインシェル"
        } else if self.direct_exec {
            "直接実行"
        } else {
            "WSL 標準シェル"
        }
    }
}
fn shell_word(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\"'\"'"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wsl_argv_preserves_linux_paths_and_each_argument_without_shell_evaluation() {
        let wsl = WslSettings {
            distribution: "Ubuntu".into(),
            user: "dev".into(),
            direct_exec: true,
            ..Default::default()
        };
        let args = vec![
            "".into(),
            "日本語 file".into(),
            "a\"b".into(),
            "$(touch unexpected)".into(),
        ];
        let expected: Vec<String> = vec![
            "--distribution",
            "Ubuntu",
            "--user",
            "dev",
            "--cd",
            "/home/dev/project space",
            "--exec",
            "ls",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain(args.clone())
        .collect();
        assert_eq!(
            wsl.arguments("ls", &args, "/home/dev/project space"),
            expected
        );
        wsl.validate("ls", "/home/dev/project space").unwrap();
        assert_eq!(
            WslSettings {
                direct_exec: true,
                ..Default::default()
            }
            .arguments("/bin/true", &[], ""),
            ["--exec", "/bin/true"]
        );
    }
    #[test]
    fn standard_wsl_shell_is_default_and_quotes_each_word() {
        let settings = WslSettings::default();
        let args = vec![
            "".into(),
            "a'\"b".into(),
            "$(touch unexpected)".into(),
            "a;b&c".into(),
        ];
        assert_eq!(
            settings.arguments("code", &args, ""),
            [
                "--",
                "'code' '' 'a'\"'\"'\"b' '$(touch unexpected)' 'a;b&c'"
            ]
        );
        assert_eq!(settings.execution_mode(), "WSL 標準シェル");
    }
    #[test]
    fn legacy_entries_default_to_windows_and_wsl_recipes_round_trip() {
        let entry = super::super::WorkspaceEntry {
            executable: "C:\\missing\\app.exe".into(),
            ..Default::default()
        };
        let text = toml::to_string(&entry).unwrap();
        assert!(!text.contains("wsl"));
        assert_eq!(
            toml::from_str::<super::super::WorkspaceEntry>(&text).unwrap(),
            entry
        );
        let entry = super::super::WorkspaceEntry {
            executable: "ls".into(),
            working_directory: "/tmp".into(),
            wsl: Some(WslSettings::default()),
            ..Default::default()
        };
        let mut workspace = super::super::Workspace::new("WSL");
        workspace.entries.push(entry.clone());
        super::super::validate_workspaces(&[workspace]).unwrap();
        assert_eq!(
            toml::from_str::<super::super::WorkspaceEntry>(&toml::to_string(&entry).unwrap())
                .unwrap(),
            entry
        );
        assert!(
            WslSettings::default()
                .validate("C:\\app.exe", "C:\\folder")
                .is_err()
        );
        assert!(
            WslSettings {
                timeout_secs: 0,
                ..Default::default()
            }
            .validate("ls", "/")
            .is_err()
        );
    }
    #[test]
    fn login_shell_uses_positional_argv_and_placement_requires_windows_target() {
        let wsl = WslSettings {
            login_shell: true,
            place_window: true,
            ..Default::default()
        };
        let args = vec!["$(touch unexpected)".into(), "a'\"b".into(), "".into()];
        let mut expected = vec![
            "--exec",
            "/bin/bash",
            "-lc",
            "exec \"$@\"",
            "maximdock",
            "code",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        expected.extend(args.clone());
        assert_eq!(wsl.arguments("code", &args, ""), expected);
        let mut workspace = super::super::Workspace::new("WSL GUI");
        workspace.entries.push(super::super::WorkspaceEntry {
            executable: "code".into(),
            wsl: Some(wsl),
            ..Default::default()
        });
        assert!(super::super::validate_workspaces(std::slice::from_ref(&workspace)).is_err());
        workspace.entries[0].window_executable = "C:\\apps\\Code.exe".into();
        super::super::validate_workspaces(&[workspace]).unwrap();
        let old: WslSettings = toml::from_str("timeout_secs = 60").unwrap();
        assert!(!old.login_shell && !old.place_window);
    }
}
