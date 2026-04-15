use crate::database::ObjectId;
use gio::glib;
use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, glib::Boxed)]
#[boxed_type(name = "ObjectIds")]
pub struct ObjectIds(Vec<ObjectId>, HashSet<String>);

impl ObjectIds {
    pub fn new() -> Self {
        ObjectIds::default()
    }

    pub fn mark_entry_for_removal(&mut self, uuid: String) {
        self.1.insert(uuid);
    }

    pub fn entries_to_remove(&self) -> &HashSet<String> {
        &self.1
    }
}

impl IntoIterator for ObjectIds {
    type Item = ObjectId;
    type IntoIter = <Vec<ObjectId> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for ObjectIds {
    type Target = Vec<ObjectId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ObjectIds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
