use std::path::PathBuf;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    config_paths: Vec<PathBuf>,
    resource_names: ResourceNames,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceNames {
    log_toml: String,
    settings_json: String,
    data_json: String,
    icons_dir: String,
}

impl Default for ResourceNames {
    fn default() -> Self {
        ResourceNames {
            log_toml: env!("RESOURCE_LOG_FILE").to_string(),
            settings_json: env!("RESOURCE_SETTINGS_FILE").to_string(),
            data_json: env!("RESOURCE_DATA_FILE").to_string(),
            icons_dir: "icons".to_string(),
        }
    }
}

#[allow(dead_code)]
impl ResourceNames {
    pub fn log_toml(&self) -> String {
        self.log_toml.clone()
    }

    pub fn settings_json(&self) -> String {
        self.settings_json.clone()
    }

    pub fn data_json(&self) -> String {
        self.data_json.clone()
    }

    pub fn icons_dir(&self) -> String {
        self.icons_dir.clone()
    }
}

impl Resources {

    pub fn new(config_paths: Vec<PathBuf>) -> Self {
        Resources { config_paths, resource_names: ResourceNames::default() }
    }

    pub fn names(&self) -> ResourceNames {
        self.resource_names.clone()
    }

    pub fn file(&self, file_name: &str) -> Option<PathBuf> {
        for path in &self.config_paths {
            let file_path = path.join(file_name);
            if file_path.exists() {
                return Some(file_path);
            }
        }
        None
    }

    pub fn icon(&self, icon_file: &str) -> Option<PathBuf> {
        let icon_file = format!("{}/{}", self.resource_names.icons_dir, icon_file);
        self.file(&icon_file)
    }

    pub fn log_toml(&self) -> Option<PathBuf> {
        self.file(&self.resource_names.log_toml)
    }

    pub fn settings_json(&self) -> Option<PathBuf> {
        self.file(&self.resource_names.settings_json)
    }

    pub fn settings_json_or(&self) -> PathBuf {
        self.file(&self.resource_names.settings_json).unwrap_or_else(|| {
            self.config_paths[0].join(&self.resource_names.settings_json)
        })
    }

    pub fn new_file(&self, file_name: &str) -> Option<PathBuf> {
        if self.file(file_name).is_none() {
            Some(self.config_paths[0].join(file_name))
        } else {
            None
        }
    }

    pub fn new_icon(&self, icon_file: &str) -> Option<PathBuf> {
        let icon_file = format!("{}/{}", self.resource_names.icons_dir, icon_file);
        self.new_file(&icon_file)
    }

    pub fn rename_icon(&self, existing_name: &str, new_name: &str) -> Option<PathBuf> {
        if let Some(existing_path) = self.icon(existing_name) {
            if let Some(new_path) = self.new_icon(new_name).or_else(|| self.icon(new_name)) {
                if existing_path.parent() == new_path.parent() {
                    match std::fs::rename(&existing_path, &new_path) {
                        Ok(()) => return Some(new_path),
                        Err(_) => {},
                    }
                }
            }
        }
        None
    }


    pub fn detected_icon(&self, process_name: String) -> DetectedIcon {
        DetectedIcon::new(self.clone(), process_name)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedIcon {
    resources: Resources,
    name: String,
}

impl DetectedIcon {

    fn new(resources: Resources, process_name: String) -> Self {
        DetectedIcon {
            resources,
            name: format!("detected_{}.png", slugify_process_name(&process_name))
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn exists(&self) -> bool {
        self.resources.icon(&self.name()).is_some()
    }

    pub fn final_name(&self) -> String {
        self.name.trim_start_matches("detected_").to_string()
    }

    pub fn finalize(&self) -> Option<PathBuf> {
        // remove "detected_" prefix when renaming
        let new_name = self.name.trim_start_matches("detected_");
        self.resources.rename_icon(&self.name(), new_name)

    }

    pub fn path(&self) -> PathBuf {
        self.resources.icon(&self.name())
            .or_else(|| self.resources.new_icon(&self.name()))
            .ok_or_else(|| format!("Source file path not found in resources: {}", &self.name()))
            .unwrap()
    }

    pub fn as_option(&self) -> Option<DetectedIcon> {
        if self.exists() {
            Some(self.clone())
        } else {
            None
        }
    }

}

pub fn slugify_process_name(process_name: &str) -> String {
    process_name.to_lowercase().replace(".exe", "").replace(" ", "_").replace("-", "_").replace("+", "_")
}
