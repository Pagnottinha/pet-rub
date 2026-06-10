use serde::{Deserialize, Serialize};
use strum::Display;

use crate::components::{lei, patchsets};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    SwitchModeHome,
    SwitchModeConfig,
    EditFile(String, Option<Box<Action>>),
    ValidateAndSaveConfig,
    LeiSetMode(lei::LocalMode),
    LeiFetchPatchsets,
    // TODO: Implement Lei action to have local public inbox for faster loadings
    PatchsetsList(String),
    PatchsetsAddIndex,
    PatchsetsSubIndex,
    PatchsetsThread,
    PatchsetsSetMode(patchsets::LocalMode),
    KtreeApply(String),
}
