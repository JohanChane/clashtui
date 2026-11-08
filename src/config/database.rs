/// Group name → provider_name → URL.
pub type ProxyProviderGroups =
    std::collections::HashMap<String, std::collections::BTreeMap<String, String>>;

#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub dtype: ProfileType,
    pub no_pp: bool,
    pub update_with_proxy: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            dtype: ProfileType::File,
            no_pp: false,
            update_with_proxy: false,
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug, Default)]
pub struct ProfileData {
    pub dtype: ProfileType,
    pub no_pp: bool,
    pub update_with_proxy: bool,
}

impl ProfileData {
    pub fn new(dtype: ProfileType) -> Self {
        Self {
            dtype,
            no_pp: false,
            update_with_proxy: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ProfileType {
    #[default]
    File,
    Url(String),
    Template {
        template: String,
    },
    Singbox,
}

impl serde::Serialize for ProfileType {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            ProfileType::File => serializer.serialize_unit_variant("ProfileType", 0, "File"),
            ProfileType::Url(url) => {
                serializer.serialize_newtype_variant("ProfileType", 1, "Url", url)
            }
            ProfileType::Template { template } => {
                #[derive(serde::Serialize)]
                struct TplHelper<'a> {
                    template: &'a str,
                }
                serializer.serialize_newtype_variant(
                    "ProfileType",
                    2,
                    "Template",
                    &TplHelper { template },
                )
            }
            ProfileType::Singbox => serializer.serialize_unit_variant("ProfileType", 3, "Singbox"),
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
            #[serde(rename = "Template")]
            Template {
                template: String,
                #[serde(default)]
                proxy_provider_groups: Option<ProxyProviderGroups>,
            },
            #[allow(dead_code)]
            #[serde(rename = "Generated")]
            Generated(String),
            #[serde(rename = "Singbox")]
            Singbox,
        }

        let wire = Wire::deserialize(deserializer)?;
        Ok(match wire {
            Wire::File => ProfileType::File,
            Wire::Url(s) => ProfileType::Url(s),
            Wire::Template {
                template,
                proxy_provider_groups,
            } => {
                if let Some(groups) = proxy_provider_groups
                    && !groups.is_empty()
                {
                    log::warn!(
                        "Migrating legacy Template profile '{template}': proxy_provider_groups found in database, will write to template file."
                    );
                    PENDING_TEMPLATE_MIGRATIONS
                        .lock()
                        .unwrap()
                        .push((template.clone(), groups));
                }
                ProfileType::Template { template }
            }
            Wire::Generated(name) => {
                log::warn!("Migrating deprecated ProfileType::Generated({name}) to Template.");
                ProfileType::Template { template: name }
            }
            Wire::Singbox => ProfileType::Singbox,
        })
    }
}

/// Queue for legacy Template entries with embedded proxy_provider_groups.
/// Drained after ProfileManager::from_file() to write groups into template files.
pub static PENDING_TEMPLATE_MIGRATIONS: std::sync::LazyLock<
    std::sync::Mutex<Vec<(String, ProxyProviderGroups)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

impl serde::Serialize for ProfileData {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let fields = 2 + self.update_with_proxy as usize;
        let mut map = serializer.serialize_map(Some(fields))?;
        map.serialize_entry("dtype", &self.dtype)?;
        map.serialize_entry("no_pp", &self.no_pp)?;
        if self.update_with_proxy {
            map.serialize_entry("update_with_proxy", &self.update_with_proxy)?;
        }
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for ProfileData {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value =
            serde_yml::Value::deserialize(deserializer).map_err(serde::de::Error::custom)?;

        if let serde_yml::Value::Mapping(map) = value {
            let dtype = map
                .get(serde_yml::Value::String("dtype".into()))
                .map(|v| serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom))
                .transpose()?
                .unwrap_or(ProfileType::File);
            let no_pp = map
                .get(serde_yml::Value::String("no_pp".into()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let update_with_proxy = map
                .get(serde_yml::Value::String("update_with_proxy".into()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(ProfileData {
                dtype,
                no_pp,
                update_with_proxy,
            })
        } else {
            let dtype = serde_yml::from_value(value).map_err(serde::de::Error::custom)?;
            Ok(ProfileData {
                dtype,
                no_pp: false,
                update_with_proxy: false,
            })
        }
    }
}

type ProfileDataMap = std::collections::HashMap<String, ProfileData>;

#[cfg_attr(test, derive(PartialEq))]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct CoreProfileData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cur_profile: Option<String>,
    #[serde(default)]
    pub profiles: ProfileDataMap,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
/// manage profiles with mihomo/singbox sections
pub struct ProfileManager {
    #[serde(default)]
    pub core_type: crate::config::CoreType,
    #[serde(default)]
    pub mihomo: CoreProfileData,
    #[serde(default)]
    pub singbox: CoreProfileData,
    /// Persist this selection while the current session keeps using its original core.
    #[serde(skip)]
    next_core_type: Option<crate::config::CoreType>,
}
impl ProfileManager {
    pub fn to_file(&self) -> anyhow::Result<()> {
        self.save_to(&super::config_dir_path().join(super::defs::DATA_FILE))
    }

    pub fn switch_core(&mut self, core: crate::config::CoreType) -> anyhow::Result<()> {
        self.switch_core_at(core, &super::config_dir_path().join(super::defs::DATA_FILE))
    }

    fn switch_core_at(
        &mut self,
        core: crate::config::CoreType,
        path: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut next = self.clone();
        next.next_core_type = Some(core);
        next.save_to(path)?;
        // Background saves and the final exit save must retain the new selection.
        self.next_core_type = Some(core);
        Ok(())
    }

    fn save_to(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use anyhow::Context;
        use std::io::Write;

        // Preserve databases stored behind a symlink when replacing the file atomically.
        let resolved;
        let path = if std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_symlink()) {
            resolved = std::fs::canonicalize(path)
                .with_context(|| format!("Failed to resolve {}", path.display()))?;
            resolved.as_path()
        } else {
            path
        };
        let mut snapshot = self.clone();
        snapshot.core_type = self.next_core_type.unwrap_or(self.core_type);
        let text = serde_yml::to_string(&snapshot)?;
        let temp = path.with_extension(format!("{}.tmp", fastrand::u64(..)));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .with_context(|| format!("Failed to create {}", temp.display()))?;
        let result = (|| -> anyhow::Result<()> {
            if let Ok(metadata) = std::fs::metadata(path) {
                file.set_permissions(metadata.permissions())?;
            }
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result.with_context(|| format!("Failed to save {}", path.display()))
    }

    pub fn contains_in_singbox(&self, name: &str) -> bool {
        self.singbox.profiles.contains_key(name)
    }

    pub fn insert<S: AsRef<str>>(&mut self, name: S, dtype: ProfileType) -> Option<Profile> {
        let db = &mut match dtype {
            ProfileType::Singbox => &mut self.singbox,
            ProfileType::Template { .. } if self.core_type == crate::config::CoreType::Singbox => {
                &mut self.singbox
            }
            ProfileType::Url(_) if self.core_type == crate::config::CoreType::Singbox => {
                &mut self.singbox
            }
            _ => &mut self.mihomo,
        };
        db.profiles
            .insert(name.as_ref().into(), ProfileData::new(dtype))
            .map(|data| Profile {
                name: name.as_ref().to_string(),
                dtype: data.dtype,
                no_pp: data.no_pp,
                update_with_proxy: data.update_with_proxy,
            })
    }
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        let name = name.as_ref();
        self.mihomo
            .profiles
            .get(name)
            .cloned()
            .or_else(|| self.singbox.profiles.get(name).cloned())
            .map(|data| Profile {
                name: name.to_string(),
                dtype: data.dtype,
                no_pp: data.no_pp,
                update_with_proxy: data.update_with_proxy,
            })
    }
    /// return profile names for the active core only
    pub fn all_for_core(&self) -> Vec<String> {
        match self.core_type {
            crate::config::CoreType::Mihomo => self.mihomo.profiles.keys().cloned().collect(),
            crate::config::CoreType::Singbox => self.singbox.profiles.keys().cloned().collect(),
        }
    }
    pub fn remove<S: AsRef<str>>(&mut self, name: S) -> Option<Profile> {
        let name = name.as_ref();
        let from_mihomo = self.mihomo.profiles.remove(name).map(|data| Profile {
            name: name.to_string(),
            dtype: data.dtype,
            no_pp: data.no_pp,
            update_with_proxy: data.update_with_proxy,
        });
        if from_mihomo.is_some() {
            return from_mihomo;
        }
        self.singbox.profiles.remove(name).map(|data| Profile {
            name: name.to_string(),
            dtype: data.dtype,
            no_pp: data.no_pp,
            update_with_proxy: data.update_with_proxy,
        })
    }
    pub fn get_current(&self) -> Option<Profile> {
        match self.core_type {
            crate::config::CoreType::Mihomo => {
                let name = self.mihomo.cur_profile.as_ref()?;
                self.get(name)
            }
            crate::config::CoreType::Singbox => {
                let name = self.singbox.cur_profile.as_ref()?;
                self.get(name)
            }
        }
    }
    pub fn set_current(&mut self, pf: Profile) {
        let name = pf.name.clone();
        assert!(
            self.get(&name).is_some(),
            "Selected profile not found in database"
        );
        match self.core_type {
            crate::config::CoreType::Mihomo => self.mihomo.cur_profile = Some(name),
            crate::config::CoreType::Singbox => self.singbox.cur_profile = Some(name),
        }
    }
    pub fn set_no_pp<S: AsRef<str>>(&mut self, name: S, no_pp: bool) {
        let name = name.as_ref();
        if let Some(data) = self.mihomo.profiles.get_mut(name) {
            data.no_pp = no_pp;
        } else if let Some(data) = self.singbox.profiles.get_mut(name) {
            data.no_pp = no_pp;
        }
    }
    pub fn set_update_with_proxy<S: AsRef<str>>(&mut self, name: S, val: bool) {
        let name = name.as_ref();
        if let Some(data) = self.mihomo.profiles.get_mut(name) {
            data.update_with_proxy = val;
        } else if let Some(data) = self.singbox.profiles.get_mut(name) {
            data.update_with_proxy = val;
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn switch_core_persists_without_changing_runtime_and_survives_later_saves() {
        use super::*;
        use crate::config::CoreType;
        let root = std::env::temp_dir().join(format!("clashtui-switch-{}", fastrand::u64(..)));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("clashtui.db");
        for (src, dst) in [
            (CoreType::Mihomo, CoreType::Singbox),
            (CoreType::Singbox, CoreType::Mihomo),
        ] {
            let mut db = ProfileManager {
                core_type: src,
                ..Default::default()
            };
            db.mihomo.cur_profile = Some("mihomo.yaml".into());
            db.singbox.cur_profile = Some("singbox.json".into());
            db.save_to(&path).unwrap();
            db.switch_core_at(dst, &path).unwrap();
            assert_eq!(db.core_type, src);
            db.insert("later-save.yaml", ProfileType::File);
            db.save_to(&path).unwrap();
            let saved: ProfileManager =
                serde_yml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(saved.core_type, dst);
            assert_eq!(saved.mihomo, db.mihomo);
            assert_eq!(saved.singbox, db.singbox);
            assert!(saved.next_core_type.is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_switch_does_not_commit_selection_or_leave_temporary_files() {
        use super::*;
        use crate::config::CoreType;
        let root = std::env::temp_dir().join(format!("clashtui-switch-fail-{}", fastrand::u64(..)));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("clashtui.db");
        let mut db = ProfileManager::default();
        db.save_to(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        // Force rename failure after writing the temporary file, on every platform.
        let blocked = root.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        assert!(db.switch_core_at(CoreType::Singbox, &blocked).is_err());
        assert_eq!(db.core_type, CoreType::Mihomo);
        assert!(db.next_core_type.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        db.save_to(&path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }

    use super::*;
    #[test]
    fn serde_template_deserialized_as_template() {
        let yaml = r#"core_type: mihomo
mihomo:
  profiles:
    pf1: File
    pf2: !Url "https://raw.com"
    pf3: !Template {template: tpl.yaml}
singbox:
  profiles: {}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(
            db.mihomo.profiles.get("pf1").unwrap().dtype,
            ProfileType::File
        );
        assert!(!db.mihomo.profiles.get("pf1").unwrap().no_pp);
        assert_eq!(
            db.mihomo.profiles.get("pf2").unwrap().dtype,
            ProfileType::Url("https://raw.com".to_string())
        );
        assert!(!db.mihomo.profiles.get("pf2").unwrap().no_pp);
        assert_eq!(
            db.mihomo.profiles.get("pf3").unwrap().dtype,
            ProfileType::Template {
                template: "tpl.yaml".into(),
            }
        );
        assert!(!db.mihomo.profiles.get("pf3").unwrap().no_pp);
    }
    #[test]
    fn serde_generated_migrated_to_template() {
        let yaml = r#"core_type: mihomo
mihomo:
  profiles:
    pf1: !Generated "my-tpl.yaml"
singbox:
  profiles: {}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert_eq!(
            db.mihomo.profiles.get("pf1").unwrap().dtype,
            ProfileType::Template {
                template: "my-tpl.yaml".into(),
            }
        );
        assert!(!db.mihomo.profiles.get("pf1").unwrap().no_pp);
    }
    #[test]
    fn serde_roundtrip_file_and_url() {
        let mut db = ProfileManager::default();
        db.insert("pf1", ProfileType::File);
        db.insert("pf2", ProfileType::Url("https://raw.com".to_string()));
        db.insert(
            "pf3",
            ProfileType::Template {
                template: "my-tpl.yaml".into(),
            },
        );
        let serialized = serde_yml::to_string(&db).unwrap();
        let deser: ProfileManager = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(db, deser);
    }
    #[test]
    fn new_format_preserves_no_pp() {
        let yaml = r#"core_type: mihomo
mihomo:
  profiles:
    pf1: {dtype: File, no_pp: true}
    pf2: {dtype: !Url "https://example.com", no_pp: false}
singbox:
  profiles: {}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert!(db.mihomo.profiles.get("pf1").unwrap().no_pp);
        assert!(!db.mihomo.profiles.get("pf2").unwrap().no_pp);
    }
    #[test]
    fn new_format_missing_no_pp_defaults_false() {
        let yaml = r#"core_type: mihomo
mihomo:
  profiles:
    pf1: {dtype: File}
singbox:
  profiles: {}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert!(!db.mihomo.profiles.get("pf1").unwrap().no_pp);
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
    #[test]
    fn backward_compat_missing_update_with_proxy_defaults_false() {
        let yaml = r#"core_type: mihomo
mihomo:
  profiles:
    pf1: {dtype: File, no_pp: true}
singbox:
  profiles: {}
"#;
        let db: ProfileManager = serde_yml::from_str(yaml).unwrap();
        assert!(!db.mihomo.profiles.get("pf1").unwrap().update_with_proxy);
    }

    #[test]
    fn set_update_with_proxy_toggles_and_persists() {
        let mut db = ProfileManager::default();
        db.insert("pf1", ProfileType::File);

        db.set_update_with_proxy("pf1", true);
        assert!(db.get("pf1").unwrap().update_with_proxy);

        db.set_update_with_proxy("pf1", false);
        assert!(!db.get("pf1").unwrap().update_with_proxy);
    }

    #[test]
    fn update_with_proxy_roundtrip() {
        let mut db = ProfileManager::default();
        db.insert("pf1", ProfileType::File);
        db.set_update_with_proxy("pf1", true);
        db.insert("pf2", ProfileType::Url("https://example.com".into()));
        db.set_update_with_proxy("pf2", false);

        let serialized = serde_yml::to_string(&db).unwrap();
        let deser: ProfileManager = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(db, deser);
    }
}
