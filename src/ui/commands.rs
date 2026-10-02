use crate::core::Config;

pub(super) enum Command {
    Launch(uuid::Uuid),
    PickFiles,
    PickFolder,
    Apply(Config),
    OpenDropped {
        id: uuid::Uuid,
        paths: Vec<std::path::PathBuf>,
    },
    RejectDrop,
    Reorder {
        id: uuid::Uuid,
        before: Option<uuid::Uuid>,
    },
    DiscardSettings,
    ReloadIcons,
    BackUpInvalid,
    OpenConfigFolder,
    OpenSettings,
    Show,
    Hide,
    Quit,
}
