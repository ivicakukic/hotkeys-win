// use std::env;
use std::fs;
use std::path::Path;
use toml::Value;

use windres;
use winres;
use winapi;

// Developer Resource Configuration Example
// Copy this content to 'dev-resources.toml' and modify as needed (is .gitignored)
//
// [resources]
// # Override default resource file names for development
// # These paths are relative to the config directory (resources/)
//
// # Default: "log.toml"
// log_file = "dev-log.toml"
//
// # Default: "settings.json"
// settings_file = "dev-settings.json"
//
// # Default: "data.json"
// data_file = "dev-data.json"


#[derive(Debug)]
struct ResourceConfig {
    log_file: String,
    settings_file: String,
    data_file: String,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            log_file: "log.toml".to_string(),
            settings_file: "settings.json".to_string(),
            data_file: "data.json".to_string(),
        }
    }
}

fn load_resource_config() -> ResourceConfig {
    let config_path = Path::new("dev-resources.toml");

    if !config_path.exists() {
        println!("cargo:warning=No dev-resources.toml found, using defaults");
        return ResourceConfig::default();
    }

    let config_content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) => {
            println!("cargo:warning=Failed to read dev-resources.toml: {}, using defaults", e);
            return ResourceConfig::default();
        }
    };

    let config_toml: Value = match config_content.parse() {
        Ok(toml) => toml,
        Err(e) => {
            println!("cargo:warning=Failed to parse dev-resources.toml: {}, using defaults", e);
            return ResourceConfig::default();
        }
    };

    let resources = match config_toml.get("resources") {
        Some(resources) => resources,
        None => {
            println!("cargo:warning=No [resources] section in dev-resources.toml, using defaults");
            return ResourceConfig::default();
        }
    };

    let mut config = ResourceConfig::default();

    if let Some(log_file) = resources.get("log_file").and_then(|v| v.as_str()) {
        config.log_file = log_file.to_string();
    }

    if let Some(settings_file) = resources.get("settings_file").and_then(|v| v.as_str()) {
        config.settings_file = settings_file.to_string();
    }

    if let Some(data_file) = resources.get("data_file").and_then(|v| v.as_str()) {
        config.data_file = data_file.to_string();
    }

    config
}

fn main() {
    // Resource configuration setup
    println!("cargo:rerun-if-changed=dev-resources.toml");

    let config = load_resource_config();

    // Set environment variables for compile-time access
    println!("cargo:rustc-env=RESOURCE_LOG_FILE={}", config.log_file);
    println!("cargo:rustc-env=RESOURCE_SETTINGS_FILE={}", config.settings_file);
    println!("cargo:rustc-env=RESOURCE_DATA_FILE={}", config.data_file);

    // Windows-specific resource copying and compilation
    // let target_resources_path = Path::new(&env::var("OUT_DIR").unwrap()).join("../../../resources");

    // if target_resources_path.exists() {
    //     fs::remove_dir_all(&target_resources_path).expect("Failed to remove resource directory");
    // }
    // fs::create_dir(&target_resources_path).expect("Failed to create resource directory");

    // Use configured file names for copying
    // for file_name in vec![&config.log_file, &config.settings_file] {
    //     let source_path = Path::new("resources").join(file_name);
    //     if source_path.exists() {
    //         fs::copy(
    //             source_path,
    //             target_resources_path.join(file_name),
    //         )
    //         .unwrap();
    //     }
    // }

    windres::Build::new().compile("resources/windres/resources.rc").unwrap();

    winres::WindowsResource::new()
        .set_language(winapi::um::winnt::MAKELANGID(
            winapi::um::winnt::LANG_ENGLISH,
            winapi::um::winnt::SUBLANG_ENGLISH_US
        ))
        .set_manifest_file("resources/windres/app.manifest")
        .compile().unwrap();
}

