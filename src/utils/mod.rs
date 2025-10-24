use zed_extension_api::{self as zed};

pub fn get_executable(name: &str) -> String {
    let (platform, _) = zed::current_platform();
 
    match platform {
        zed_extension_api::Os::Windows => format!("{name}.exe"),
        _ => name.to_string(),
    }
}

pub fn get_nuget_asset_name(package_id: String, version: String) -> String {
    format!(
        "{package_id}.{version}.{extension}",
        package_id = package_id.clone(),
        version = version,
        extension = "nupkg",
    )
}

pub fn get_version_dir(package_id: String, version: String) -> String {
    format!(
        "{package_id}-{version}",
        package_id = package_id,
        version = version,
    )
}