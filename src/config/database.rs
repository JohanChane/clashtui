#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub dtype: ProfileType,
    pub no_pp: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            dtype: ProfileType::File,
            no_pp: false,
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug, Default)]
pub struct ProfileData {
    pub dtype: ProfileType,
    pub no_pp: bool,
}

impl ProfileData {
    pub fn new(dtype: ProfileType) -> Self {
        Self { dtype, no_pp: false }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum ProfileType {
    File,
    Url(String),
}

impl Default for ProfileType {
    fn default() -> Self {
        ProfileType::File
    }
}

impl serde::Serialize for ProfileType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            ProfileType::File => serializer.serialize_unit_variant("ProfileType", 0, "File"),
            ProfileType::Url(url) => {
                serializer.serialize_newtype_variant("ProfileType", 1, "Url", url)
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for ProfileType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        enum Wire {
            #[serde(rename = "File")]
            File,
            #[serde(rename = "Url")]
            Url(String),
            #[allow(dead_code)]
            #[serde(rename = "Template")]
            Template {
                template: String,
                #[serde(default)]
                urls: Vec<String>,
            },
            #[allow(dead_code)]
            #[serde(rename = "Generated")]
            Generated(String),
        }

        let wire = Wire::deserialize(deserializer)?;
        Ok(match wire {
            Wire::File => ProfileType::File,
            Wire::Url(s) => ProfileType::Url(s),
            Wire::Template { template, .. } => {
                log::warn!(
                    "Migrating deprecated ProfileType::Template({template}) to File."
                );
                ProfileType::File
            }
            Wire::Generated(name) => {
                log::warn!(
                    "Migrating deprecated ProfileType::Generated({name}) to File."
                );
                ProfileType::File
            }
        })
    }
}

impl serde::Serialize for ProfileData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("dtype", &self.dtype)?;
        map.serialize_entry("no_pp", &self.no_pp)?;
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for ProfileData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = serde_yml::Value::deserialize(deserializer).map_err(serde::de::Error::custom)?;

        if let serde_yml::Value::Mapping(map) = value {
            let dtype = map
                .get(&serde_yml::Value::String("dtype".into()))
                .map(|v| serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom))
                .transpose()?
                .unwrap_or(ProfileType::File);
            let no_pp = map
                .get(&serde_yml::Value::String("no_pp".into()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(ProfileData { dtype, no_pp })
        } else {
            let dtype = serde_yml::from_value(value).map_err(serde::de::Error::custom)?;
            Ok(ProfileData { dtype, no_pp: false })
        }
    }
}

type ProfileDataBase = std::collections::HashMap<String, ProfileData>;

#[cfg_attr(test, derive(PartialEq))]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
/// manage profiles
pub struct ProfileManager {
    current_profile: String,
    profiles: ProfileDataBase,
}
impl ProfileManager {
    pub fn insert<S: AsRef<str>>(&mut self, name: S, dtype: ProfileType) -> Option<Profile> {
        self.profiles
            .insert(name.as_ref().into(), ProfileData::new(dtype))
            .map(|data| Profile {
                name: name.as_ref().to_string(),
                dtype: data.dtype,
                no_pp: data.no_pp,
            })
    }
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.profiles
            .get(name.as_ref())
            .cloned()
            .map(|data| Profile {
                name: name.as_ref().to_string(),
                dtype: data.dtype,
                no_pp: data.no_pp,
            })
    }
    /// return all profile names
    pub fn all(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
    pub fn remove<S: AsRef<str>>(&mut self, name: S) -> Option<Profile> {
        self.profiles.remove(name.as_ref()).map(|data| Profile {
            name: name.as_ref().to_string(),
            dtype: data.dtype,
            no_pp: data.no_pp,
        })
    }
    pub fn get_current(&self) -> Option<Profile> {
        self.get(self.current_profile.as_str())
    }
    pub fn set_current(&mut self, pf: Profile) {
        assert!(
            self.get(&pf.name).is_some(),
            "Selected profile not found in database"
        );
        self.current_profile = pf.name;
    }
    pub fn set_no_pp<S: AsRef<str>>(&mut self, name: S, no_pp: bool) {
        if let Some(data) = self.profiles.get_mut(name.as_ref()) {
            data.no_pp = no_pp;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn serde_template_migrated_to_file() {
        let yaml = r#"current_profile: ''
profiles:
  pf1: File
  pf2: !Url "https://raw.com"
  pf3: !Template {template: tpl.yaml, urls: ["https://a.com"]}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(db.profiles.get("pf1").unwrap().dtype, ProfileType::File);
        assert_eq!(db.profiles.get("pf1").unwrap().no_pp, false);
        assert_eq!(
            db.profiles.get("pf2").unwrap().dtype,
            ProfileType::Url("https://raw.com".to_string())
        );
        assert_eq!(db.profiles.get("pf2").unwrap().no_pp, false);
        assert_eq!(db.profiles.get("pf3").unwrap().dtype, ProfileType::File);
        assert_eq!(db.profiles.get("pf3").unwrap().no_pp, false);
    }
    #[test]
    fn serde_generated_migrated_to_file() {
        let yaml = r#"current_profile: ''
profiles:
  pf1: !Generated "my-tpl.yaml"
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(db.profiles.get("pf1").unwrap().dtype, ProfileType::File);
        assert_eq!(db.profiles.get("pf1").unwrap().no_pp, false);
    }
    #[test]
    fn serde_roundtrip_file_and_url() {
        let mut db = ProfileManager {
            current_profile: "".to_string().into(),
            profiles: ProfileDataBase::new().into(),
        };
        db.insert("pf1", ProfileType::File);
        db.insert("pf2", ProfileType::Url("https://raw.com".to_string()));
        let serialized = serde_yml::to_string(&db).unwrap();
        let deser: ProfileManager = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(db, deser);
    }
    #[test]
    fn backward_compat_old_format_no_pp_defaults_false() {
        let yaml = r#"current_profile: ''
profiles:
  pf1: File
  pf2: !Url "https://example.com"
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(db.profiles.get("pf1").unwrap().no_pp, false);
        assert_eq!(db.profiles.get("pf2").unwrap().no_pp, false);
    }
    #[test]
    fn new_format_preserves_no_pp() {
        let yaml = r#"current_profile: ''
profiles:
  pf1: {dtype: File, no_pp: true}
  pf2: {dtype: !Url "https://example.com", no_pp: false}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(db.profiles.get("pf1").unwrap().no_pp, true);
        assert_eq!(db.profiles.get("pf2").unwrap().no_pp, false);
    }
    #[test]
    fn new_format_missing_no_pp_defaults_false() {
        let yaml = r#"current_profile: ''
profiles:
  pf1: {dtype: File}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(db.profiles.get("pf1").unwrap().no_pp, false);
    }
    #[test]
    fn set_no_pp_toggles_and_persists() {
        let mut db = ProfileManager::default();
        db.insert("pf1", ProfileType::File);

        db.set_no_pp("pf1", true);
        assert!(db.get("pf1").unwrap().no_pp);

        db.set_no_pp("pf1", false);
        assert!(!db.get("pf1").unwrap().no_pp);
    }
    #[test]
    fn new_format_roundtrip_preserves_no_pp() {
        let mut db = ProfileManager::default();
        db.insert("pf1", ProfileType::File);
        db.set_no_pp("pf1", true);
        db.insert("pf2", ProfileType::Url("https://example.com".into()));
        db.set_no_pp("pf2", false);

        let serialized = serde_yml::to_string(&db).unwrap();
        let deser: ProfileManager = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(db, deser);
    }
}
