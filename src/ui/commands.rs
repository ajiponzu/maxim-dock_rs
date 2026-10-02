use crate::core::Config;

pub(super) enum Command {
    Launch(uuid::Uuid),
    PickFiles,
    PickFolder,
    Apply(Config),
    BackUpInvalid,
    OpenConfigFolder,
    OpenSettings,
    Show,
    Hide,
    Quit,
}
