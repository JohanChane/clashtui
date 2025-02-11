use std::cell::RefCell;

use super::{Profile, ProfileType};

type ProfileDataBase = std::collections::HashMap<String, ProfileType>;

/// manage profiles
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct ProfileManager {
    current_profile: RefCell<String>,
    profiles: RefCell<ProfileDataBase>,
}
impl ProfileManager {
    pub fn insert<S: AsRef<str>>(&self, name: S, dtype: ProfileType) -> Option<Profile> {
        self.profiles
            .borrow_mut()
            .insert(name.as_ref().into(), dtype)
            .map(|dtype| Profile {
                name: name.as_ref().to_string(),
                dtype,
            })
    }
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.profiles
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
        self.profiles.borrow().keys().cloned().collect()
    }
    pub fn remove<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.profiles
            .borrow_mut()
            .remove(name.as_ref())
            .map(|dtype| Profile {
                name: name.as_ref().to_string(),
                dtype,
            })
    }
    pub fn get_current(&self) -> Option<Profile> {
        self.get(self.current_profile.borrow().as_str())
    }
    pub fn set_current(&self, pf: Profile) {
        assert!(
            self.get(&pf.name).is_some(),
            "Selected profile not found in database"
        );
        *self.current_profile.borrow_mut() = pf.name;
    }
}
