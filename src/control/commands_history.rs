use crate::control::modify_playlist_action::ModifyPlaylistAction;

#[derive(Default)]
pub struct CommandsHistory {
    applied: Vec<ModifyPlaylistAction>,
    undone: Vec<ModifyPlaylistAction>,
}

impl CommandsHistory {
    pub fn new() -> CommandsHistory {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.applied.clear();
        self.undone.clear();
    }

    pub fn push(&mut self, entry: ModifyPlaylistAction) {
        self.undone.clear();
        self.applied.push(entry);
    }

    pub fn undo(&mut self) -> Option<ModifyPlaylistAction> {
        let elem = self.applied.pop()?;
        self.undone.push(elem.clone());
        Some(elem)
    }

    pub fn redo(&mut self) -> Option<ModifyPlaylistAction> {
        let elem = self.undone.pop()?;
        self.applied.push(elem.clone());
        Some(elem)
    }
}
