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

pub fn get_runtime_identifier() -> String {
    let (platform, arch) = zed::current_platform();

    format!(
        "{os}-{arch}",
        os = match platform {
            zed::Os::Mac => "osx",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "win",
        },
        arch = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X86 => "x86",
            zed::Architecture::X8664 => "x64",
        },
    )
}

pub fn ensure_github_release<F>(
    package: &str,
    repo: &str,
    tag: &str,
    get_package_id: F,
    download_file_type: zed::DownloadedFileType,
) -> Result<String, String>
where
    F: Fn() -> String,
{
    let package_id = get_package_id().clone();
    let download_path = get_version_dir(package_id.clone(), tag.into());

    if std::fs::metadata(&download_path).is_ok() {
        println!(
            "[zed-roslynls] {} already downloaded at: {}",
            package, download_path
        );

        let executable_path = match download_file_type {
            zed_extension_api::DownloadedFileType::Uncompressed => download_path,
            _ => {
                let current_dir = std::env::current_dir()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
 
                format!("{current_dir}/{download_path}/{package}")
            }
        };

        return Ok(executable_path);
    }

    match zed::github_release_by_tag_name(repo, tag) {
        Ok(release) => {
            let asset = release.assets.iter().find(|asset| asset.name == package_id);

            println!("[zed-roslynls] Found asset: {:?} {:?}", package_id, asset);

            if let Some(asset) = asset {
                let download_url = &asset.download_url;
                println!(
                    "[zed-roslynls] Downloading from: {}, to: {}",
                    download_url, download_path
                );

                zed::download_file(download_url, download_path.as_str(), download_file_type)
                    .map_err(|e| format!("Failed to download {package}: {e}"))?;

                match download_file_type {
                    zed_extension_api::DownloadedFileType::Uncompressed => {
                        zed::make_file_executable(download_path.as_str())
                            .expect(format!("Failed to make {package} executable").as_str());

                        Ok(download_path)
                    }
                    zed_extension_api::DownloadedFileType::GzipTar => {
                        let executable_path = format!("{download_path}/{package}");
                        zed::make_file_executable(executable_path.as_str())
                            .expect(format!("Failed to make {package} executable").as_str());
                        Ok(executable_path)
                    }
                    zed_extension_api::DownloadedFileType::Zip => Err("Not implemented".into()),
                    zed_extension_api::DownloadedFileType::Gzip => Err("Not implemented".into()),
                }
            } else {
                println!(
                    "[zed-roslynls] No suitable {package} asset found for the current platform"
                );
                Ok(package.into())
            }
        }
        Err(e) => {
            println!("[zed-roslynls] Failed to fetch {package} release info: {e}");

            Ok(package.to_string())
        }
    }
}
