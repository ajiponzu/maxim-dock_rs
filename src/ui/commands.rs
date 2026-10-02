use crate::core::Config;

pub(super) enum Command {
    Launch(uuid::Uuid),
    PickFiles,
    PickFolder,
    Apply(Config),
    DropPaths(Vec<std::path::PathBuf>),
    DiscardSettings,
    ReloadIcons,
    BackUpInvalid,
    OpenConfigFolder,
    OpenSettings,
    Show,
    Hide,
    Quit,
}
