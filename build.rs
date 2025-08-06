use std::env;
use std::fs;
use std::path::Path;

use windres;
use winres;
use winapi;

fn main() {
    let target_resources_path = Path::new(&env::var("OUT_DIR").unwrap()).join("../../../resources");

    if target_resources_path.exists() {
        fs::remove_dir_all(&target_resources_path).expect("Failed to remove resource directory");
    }
    fs::create_dir(&target_resources_path).expect("Failed to create resource directory");

    for file_name in vec!["log.toml", "settings.json"] {
        fs::copy(
            Path::new("resources").join(file_name),
            target_resources_path.join(file_name),
        )
        .unwrap();
    }

    windres::Build::new().compile("resources.rc").unwrap();

    winres::WindowsResource::new()
        .set_language(winapi::um::winnt::MAKELANGID(
            winapi::um::winnt::LANG_ENGLISH,
            winapi::um::winnt::SUBLANG_ENGLISH_US
        ))
        .set_manifest_file("resources/app.manifest")
        .set("CompanyName", "Modus Ponens d.o.o.")
        .compile().unwrap();
}
