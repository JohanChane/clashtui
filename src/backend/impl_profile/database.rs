use std::cell::RefCell;

use super::{Profile, ProfileType};

type ProfileDataBase = std::collections::HashMap<String, ProfileType>;

#[cfg_attr(test, derive(PartialEq))]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
/// manage profiles
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn serde() {
        let db = ProfileManager {
            current_profile: "".to_string().into(),
            profiles: ProfileDataBase::new().into(),
        };
        db.insert("pf1", ProfileType::File);
        db.insert("pf2", ProfileType::Generated("template1".to_string()));
        db.insert(
            "pf3",
            ProfileType::GitLab {
                url: "https://github.com".to_string(),
                token: "Token".to_string(),
            },
        );
        db.insert(
            "pf4",
            ProfileType::GitLab {
                url: "https://gitlab.com".to_string(),
                token: "Token".to_string(),
            },
        );
        db.insert("pf5", ProfileType::Url("https://raw.com".to_string()));
        let std = r#"current_profile: ''
profiles:
  pf5: !Url https://raw.com
  pf2: !Generated template1
  pf4: !GitLab
    url: https://gitlab.com
    token: Token
  pf1: File
  pf3: !GitLab
    url: https://github.com
    token: Token
"#;
        let std: ProfileManager = serde_yml::from_str(&std).unwrap();
        assert_eq!(db, std);
    }
}
