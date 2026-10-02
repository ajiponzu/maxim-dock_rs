//! Settings file-picker import: validated, ordered draft without live mutation.
use super::settings;
use crate::core::*;
use std::path::PathBuf;

/// One transaction for accepted paths; invalid/duplicate targets are reported,
/// never silently written. Callers persist the returned configuration first.
pub(super) fn stage_paths(config: &Config, paths: Vec<PathBuf>) -> (Config, Vec<String>) {
    let mut staged = config.clone();
    let mut failures = Vec::new();
    for path in paths {
        let label = settings::label_for_path(&path);
        let result = path
            .to_str()
            .ok_or("Path is not valid Unicode.")
            .and_then(|target| validate_target(target, TargetKind::Path).map(|()| target))
            .map_err(str::to_owned)
            .and_then(|target| {
                staged
                    .add_item(DockItem::new(&label, target, TargetKind::Path))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            failures.push(format!("{label}: {e}"));
        }
    }
    (staged, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_windows::ConfigStore;
    #[test]
    fn selected_batch_keeps_order_rejects_duplicates_and_missing_paths() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("日本語.txt");
        std::fs::write(&file, b"test").unwrap();
        let mut config = Config::defaults("unused".into());
        config.items.clear();
        let (staged, errors) = stage_paths(
            &config,
            vec![
                file.clone(),
                directory.path().into(),
                file.clone(),
                directory.path().join("missing"),
            ],
        );
        assert!(config.items.is_empty());
        assert_eq!(staged.items.len(), 2);
        assert_eq!(staged.items[0].target, file.to_str().unwrap());
        assert_eq!(errors.len(), 2);
        let path = directory.path().join("config.toml");
        let (mut store, _, _) = ConfigStore::load(path.clone(), config.clone());
        store.save(&staged).unwrap();
        let (_, restored, error) = ConfigStore::load(path, config);
        assert!(error.is_none());
        assert_eq!(staged, restored);
    }
}
