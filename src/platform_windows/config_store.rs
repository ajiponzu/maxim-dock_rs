use crate::core::Config;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Configuration I/O failed. Check folder permissions and disk space.")]
    Io(#[from] std::io::Error),
    #[error("Invalid TOML. The original configuration is preserved.")]
    Parse(#[source] toml::de::Error),
    #[error("Configuration is not UTF-8. The original file is preserved.")]
    Encoding,
    #[error("{0}")]
    Validation(#[from] crate::core::ValidationError),
    #[error("Cannot serialize configuration.")]
    Serialize(#[source] toml::ser::Error),
    #[error("Configuration changed outside this app. Restart to load it before saving.")]
    Conflict,
    #[error("Saving is blocked. Back up the invalid configuration explicitly first.")]
    Blocked,
    #[error("Windows could not replace the configuration. The original file is preserved.")]
    Replace(#[source] windows::core::Error),
}

pub struct ConfigStore {
    path: PathBuf,
    snapshot: Option<Vec<u8>>,
    blocked: bool,
}
impl ConfigStore {
    pub fn load(path: PathBuf, defaults: Config) -> (Self, Config, Option<StoreError>) {
        let (snapshot, read_error) = match read_optional(&path) {
            Ok(bytes) => (bytes, None),
            Err(e) => (None, Some(e)),
        };
        let result = match &snapshot {
            Some(bytes) => std::str::from_utf8(bytes)
                .map_err(|_| StoreError::Encoding)
                .and_then(|text| {
                    let config: Config = toml::from_str(text).map_err(StoreError::Parse)?;
                    config.validate()?;
                    Ok(config)
                }),
            None => Ok(defaults.clone()),
        };
        let (config, error) = match result {
            Ok(config) => (config, read_error),
            Err(e) => (defaults, Some(e)),
        };
        let blocked = error.is_some();
        (
            Self {
                path,
                snapshot,
                blocked,
            },
            config,
            error,
        )
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }
    pub fn is_missing(&self) -> bool {
        self.snapshot.is_none() && !self.blocked
    }

    fn verify_unchanged(&self) -> Result<(), StoreError> {
        if read_optional(&self.path)? != self.snapshot {
            Err(StoreError::Conflict)
        } else {
            Ok(())
        }
    }

    pub fn save(&mut self, config: &Config) -> Result<(), StoreError> {
        config.validate()?;
        if self.blocked {
            return Err(StoreError::Blocked);
        }
        self.verify_unchanged()?;
        let bytes = toml::to_string_pretty(config)
            .map_err(StoreError::Serialize)?
            .into_bytes();
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".config-{}.tmp", uuid::Uuid::new_v4()));
        let result: Result<(), StoreError> = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            self.verify_unchanged()?;
            atomic_replace(&temporary, &self.path)?;
            Ok(())
        })();
        if result.is_err()
            && let Err(e) = fs::remove_file(&temporary)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(error = %e, "could not remove configuration temporary file");
        }
        result?;
        self.snapshot = Some(bytes);
        tracing::info!("configuration saved");
        Ok(())
    }

    /// Explicit UI action: keep a byte-for-byte backup before permitting replacement.
    pub fn back_up_invalid(&mut self) -> Result<PathBuf, StoreError> {
        if !self.blocked {
            return Err(StoreError::Blocked);
        }
        self.verify_unchanged()?;
        let original = self.snapshot.as_ref().ok_or(StoreError::Blocked)?;
        let backup = self
            .path
            .with_file_name(format!("config.invalid-{}.toml", uuid::Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)?;
        file.write_all(original)?;
        file.sync_all()?;
        self.blocked = false;
        Ok(backup)
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, StoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn atomic_replace(from: &Path, to: &Path) -> Result<(), StoreError> {
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both UTF-16 buffers are NUL terminated and live for this call.
    // The temporary file is on the same volume; replacing never removes the original first.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(StoreError::Replace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_reload_replace_and_unicode_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("日本語設定/config.toml");
        let defaults = Config::defaults("unavailable-home".into());
        let (mut store, mut c, error) = ConfigStore::load(path.clone(), defaults.clone());
        assert!(error.is_none());
        store.save(&c).unwrap();
        c.dock.edge = crate::core::DockEdge::Left;
        c.items[0].label = "Renamed".into();
        c.move_item(0, 2);
        let mut workspace = crate::core::Workspace::new("保存確認");
        workspace.entries.push(crate::core::WorkspaceEntry {
            executable: "C:\\unplugged\\app.exe".into(),
            arguments: vec!["空白 を含む引数".into()],
            ..Default::default()
        });
        c.workspaces.push(workspace);
        store.save(&c).unwrap();
        let (_, restored, error) = ConfigStore::load(path, defaults);
        assert!(error.is_none());
        assert_eq!(restored, c);
        assert_eq!(
            fs::read_dir(dir.path().join("日本語設定")).unwrap().count(),
            1
        );
    }

    #[test]
    fn corrupt_files_preserved_until_explicit_backup() {
        for bytes in [b"invalid = [".as_slice(), &[0xff, 0xfe], b"version = 200"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("config.toml");
            fs::write(&path, bytes).unwrap();
            let defaults = Config::defaults("missing-home".into());
            let (mut store, c, error) = ConfigStore::load(path.clone(), defaults.clone());
            assert!(error.is_some());
            assert_eq!(c, defaults);
            assert!(matches!(store.save(&c), Err(StoreError::Blocked)));
            assert_eq!(fs::read(&path).unwrap(), bytes);
            let backup = store.back_up_invalid().unwrap();
            store.save(&c).unwrap();
            assert_eq!(fs::read(backup).unwrap(), bytes);
        }
    }

    #[test]
    fn failed_validation_and_external_edit_never_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let (mut store, mut c, _) =
            ConfigStore::load(path.clone(), Config::defaults("home".into()));
        store.save(&c).unwrap();
        let original = fs::read(&path).unwrap();
        c.dock.hot_zone_px = 0;
        assert!(store.save(&c).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        c.dock.hot_zone_px = 2;
        fs::write(&path, b"external edit").unwrap();
        assert!(matches!(store.save(&c), Err(StoreError::Conflict)));
        assert_eq!(fs::read(&path).unwrap(), b"external edit");
    }

    #[test]
    fn failed_windows_replace_preserves_original_and_cleans_temporary_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let (mut store, mut config, _) =
            ConfigStore::load(path.clone(), Config::defaults("home".into()));
        store.save(&config).unwrap();
        let original = fs::read(&path).unwrap();
        // Allow reads, deny delete sharing so MoveFileExW must fail.
        let held_file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        config.dock.edge = crate::core::DockEdge::Right;
        assert!(matches!(store.save(&config), Err(StoreError::Replace(_))));
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        drop(held_file);
        store.save(&config).unwrap();
    }
}
