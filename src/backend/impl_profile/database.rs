use std::cell::RefCell;

use super::{Profile, ProfileType};

pub type ProfileDataBase = std::collections::HashMap<String, ProfileType>;

/// manage profiles
pub struct ProfileManager {
    current: RefCell<String>,
    all: RefCell<ProfileDataBase>,
}
impl ProfileManager {
    pub fn new(current: String, all: ProfileDataBase) -> Self {
        Self {
            current: current.into(),
            all: all.into(),
        }
    }
    pub fn into_inner(self) -> (String, ProfileDataBase) {
        (self.current.into_inner(), self.all.into_inner())
    }
    pub fn insert<S: AsRef<str>>(&self, name: S, dtype: ProfileType) -> Option<Profile> {
        self.all
            .borrow_mut()
            .insert(name.as_ref().into(), dtype)
            .map(|dtype| Profile {
                name: name.as_ref().to_string(),
                dtype,
            })
    }
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.all
            .borrow()
            .get(name.as_ref())
            .cloned()
            .map(|dtype| Profile {
                name: name.as_ref().to_string(),
                dtype,
            })
    }
    /// return all profile names
    pub fn all(&self) -> Vec<String> {
        self.all.borrow().keys().cloned().collect()
    }
    pub fn remove<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.all
            .borrow_mut()
            .remove(name.as_ref())
            .map(|dtype| Profile {
                name: name.as_ref().to_string(),
                dtype,
            })
    }
    pub fn get_current(&self) -> Option<Profile> {
        self.get(self.current.borrow().as_str())
    }
    pub fn set_current(&self, pf: Profile) {
        assert!(
            self.get(&pf.name).is_some(),
            "Selected profile not in database"
        );
        *self.current.borrow_mut() = pf.name;
    }
}
