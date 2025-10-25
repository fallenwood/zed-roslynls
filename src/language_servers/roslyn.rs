use std::fs::{self, FileType};
use zed_extension_api::{
    self as zed, LanguageServerId, Result, serde_json::Map, settings::LspSettings,
};

use crate::language_servers::model::NuGetPackagesResponse;
use crate::utils;

const ORGANIZATION: &str = "azure-public";
const PROJECT: &str = "vside";
const FEED: &str = "vs-impl";
const ROSLYNLS: &str = "roslynls";
const ROSLYNLS_PATH_KEY: &str = "roslynls_path";
const ROSLYNLS_REPO: &str = "fallenwood/zed-roslynls";
const ROSLYNLS_TAG: &str = "v0.0.2";
const LANGUAGE_SERVER: &str = "Microsoft.CodeAnalysis.LanguageServer";

// Example version
// const PACKAGE_VERSION: &str = "5.1.0-1.25476.5";

pub struct Roslyn {
    cached_language_server_path: Option<String>,
    cached_roslynls_path: Option<String>,
}

impl Roslyn {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn";

    pub fn new() -> Self {
        Roslyn {
            cached_language_server_path: None,
            cached_roslynls_path: None,
        }
    }

    pub fn language_server_cmd(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree).ok();

        let roslynls_path = self.ensure_roslynls(worktree);

        self.cached_roslynls_path = Some(roslynls_path.clone());

        let binary_settings = settings.and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        if let Some(path) = binary_settings
            .and_then(|binary_settings| binary_settings.path)
            .or_else(|| {
                self.cached_language_server_path
                    .as_ref()
                    .filter(|path| fs::metadata(path).map_or(false, |stat| stat.is_file()))
                    .cloned()
            })
        {
            return Self::cmd(
                roslynls_path,
                path,
                worktree.root_path().to_string(),
                binary_args,
            );
        }

        let executable = utils::get_executable(LANGUAGE_SERVER);

        if let Some(path) = worktree.which(executable.as_str()) {
            return Self::cmd(
                roslynls_path,
                path,
                worktree.root_path().to_string(),
                binary_args,
            );
        }

        if let Some(path) = &self.cached_language_server_path
            && fs::metadata(path).map_or(false, |stat| stat.is_file())
        {
            return Self::cmd(
                roslynls_path,
                path.clone(),
                worktree.root_path().to_string(),
                binary_args,
            );
        }

        let binary_path = Self::get_langauge_server_binary_path(executable.as_str());

        if fs::metadata(binary_path.clone()).map_or(false, |stat| stat.is_file()) {
            self.cached_language_server_path = Some(binary_path.clone());

            return Self::cmd(
                roslynls_path,
                binary_path.clone(),
                worktree.root_path().to_string(),
                binary_args,
            );
        }

        let binary_path = Self::ensure_language_server()?;

        self.cached_language_server_path = Some(binary_path.clone());

        Self::cmd(
            roslynls_path,
            binary_path,
            worktree.root_path().to_string(),
            binary_args,
        )
    }

    pub fn configuration_options(
        &self,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        Ok(settings.map(|user_settings| self.transform_settings_for_roslyn(user_settings)))
    }

    fn transform_settings_for_roslyn(
        &self,
        settings: zed::serde_json::Value,
    ) -> zed::serde_json::Value {
        let mut roslyn_config = Map::new();

        if let zed::serde_json::Value::Object(settings_map) = settings {
            for (key, value) in &settings_map {
                if key == ROSLYNLS_PATH_KEY {
                    continue;
                }

                if key.contains('|') {
                    // This is already in the language|category format
                    if let zed::serde_json::Value::Object(nested_settings) = value {
                        for (nested_key, nested_value) in nested_settings {
                            // The key already contains the proper format, just add the setting
                            let roslyn_key = format!("{}.{}", key, nested_key);
                            roslyn_config.insert(roslyn_key, nested_value.clone());
                        }
                    }
                }
                // Handle direct roslyn-format settings (fallback for any other format)
                else if key.contains('.') {
                    roslyn_config.insert(key.clone(), value.clone());
                }
            }
        }

        zed::serde_json::Value::Object(roslyn_config)
    }

    fn cmd(
        roslynls_path: String,
        language_server_path: String,
        project_root: String,
        binary_args: Option<Vec<String>>,
    ) -> Result<zed::Command> {
        let default_args: Vec<String> = vec![
            "--lsp".into(),
            language_server_path,
            "--project-root".into(),
            project_root,
        ];

        return Ok(zed::Command {
            command: roslynls_path,
            args: binary_args.unwrap_or(default_args),
            env: Default::default(),
        });
    }

    fn ensure_roslynls(self: &mut Self, worktree: &zed::Worktree) -> String {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree).ok();

        let roslynls_path = settings
            .as_ref()
            .and_then(|lsp_settings| lsp_settings.settings.as_ref())
            .and_then(|lsp_settings| {
                if let zed::serde_json::Value::Object(settings_map) = lsp_settings {
                    settings_map.get(ROSLYNLS_PATH_KEY).and_then(|value| {
                        if let zed::serde_json::Value::String(path) = value {
                            Some(path.clone())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            });

        println!(
            "[zed-roslynls] roslynls_path: {}",
            roslynls_path.clone().unwrap_or("None".to_string())
        );

        if let Some(path) = roslynls_path {
            path
        } else if let Some(cached_path) = &self.cached_roslynls_path {
            cached_path.clone()
        } else {
            match zed::github_release_by_tag_name(ROSLYNLS_REPO, ROSLYNLS_TAG) {
                Ok(release) => {
                    let roslynls = Self::get_roslynls_package_id();
                    let asset = release.assets.iter().find(|asset| asset.name == roslynls);

                    println!("[zed-roslynls] Found asset: {:?} {:?}", roslynls, asset);

                    if let Some(asset) = asset {
                        let download_url = &asset.download_url;
                        let download_path =
                            utils::get_version_dir(ROSLYNLS.to_string(), ROSLYNLS_TAG.to_string());

                        if std::fs::metadata(&download_path).is_ok() {
                            println!("[zed-roslynls] roslynls already downloaded at: {}", download_path);
                            return download_path;
                        }

                        println!(
                            "[zed-roslynls] Downloading roslynls from: {}, to: {}",
                            download_url, download_path
                        );

                        zed::download_file(
                            download_url,
                            download_path.as_str(),
                            zed::DownloadedFileType::Uncompressed,
                        )
                        .expect("Failed to download roslynls");

                        zed::make_file_executable(download_path.as_str())
                            .expect("Failed to make roslynls executable");

                        download_path
                    } else {
                        println!(
                            "[zed-roslynls] No suitable roslynls asset found for the current platform"
                        );
                        roslynls
                    }
                }
                Err(e) => {
                    println!(
                        "[zed-roslynls] Failed to fetch roslynls release info: {}",
                        e
                    );
                    ROSLYNLS.to_string()
                }
            }
        }
    }

    fn get_langauge_server_binary_path(executable: &str) -> String {
        let package_id = Self::get_langauge_server_package_id();
        let version = Self::get_language_server_latest_version().unwrap();
        let runtime_identifier = Self::get_runtime_identifier();

        let version_dir = utils::get_version_dir(package_id, version);

        let current_dir = std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        format!(
            "{current_dir}/{version_dir}/content/LanguageServer/{runtime_identifier}/{executable}"
        )
    }

    fn ensure_language_server() -> Result<String, String> {
        let version = Self::get_language_server_latest_version()?;

        let executable = utils::get_executable(LANGUAGE_SERVER);

        let package_id = Self::get_langauge_server_package_id();

        let binary_path = Self::get_langauge_server_binary_path(executable.as_str());

        let asset_name = utils::get_nuget_asset_name(package_id.clone(), version.clone());

        let url = format!(
            "https://pkgs.dev.azure.com/{ORGANIZATION}/{PROJECT}/_packaging/{FEED}/nuget/v3/flat2/{package_id}/{version}/{asset_name}"
        );

        println!("[zed-roslynls] Downloading Roslyn Language Server from: {}", url.clone());

        let version_dir = utils::get_version_dir(package_id, version);

        zed::download_file(&url, &version_dir, zed::DownloadedFileType::Zip)
            .map_err(|e| format!("failed to download file: {e}"))?;

        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            if entry.file_name().to_str() != Some(&version_dir) {
                fs::remove_dir_all(entry.path()).ok();
            }
        }

        let entries = fs::read_dir(&version_dir)
            .map_err(|e| format!("failed to list version directory {e}"))?;
        let mut q = std::collections::VecDeque::from_iter(entries);
        while !q.is_empty() {
            let entry = q.pop_front().unwrap();
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            let filetype = entry
                .file_type()
                .map_err(|e| format!("failed to get file type {e}"))?;
            if filetype.is_dir() {
                let sub_entries = fs::read_dir(entry.path())
                    .map_err(|e| format!("failed to list sub-directory {e}"))?;
                for sub_entry in sub_entries {
                    q.push_back(sub_entry);
                }
            } else if filetype.is_file() {
                zed::make_file_executable(entry.path().to_str().unwrap())
                    .map_err(|e| format!("failed to make file executable {e}"))?;
            }
        }

        Ok(binary_path)
    }

    fn get_runtime_identifier() -> String {
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

    fn get_langauge_server_package_id() -> String {
        let runtime_identifier = Self::get_runtime_identifier();

        format!("{LANGUAGE_SERVER}.{runtime_identifier}")
    }

    fn get_roslynls_package_id() -> String {
        let runtime_identifier = Self::get_runtime_identifier();

        format!("{ROSLYNLS}-{runtime_identifier}")
    }

    fn get_language_server_latest_version() -> Result<String, String> {
        let package_id = Self::get_langauge_server_package_id();
        let url = format!(
            "https://feeds.dev.azure.com/{ORGANIZATION}/{PROJECT}/_apis/packaging/feeds/{FEED}/packages?packageNameQuery={package_id}&api-version=6.0-preview.1",
        );

        println!(
            "[zed-roslynls] Fetching latest Roslyn Language Server version from: {}",
            url.clone()
        );

        let request = zed::http_client::HttpRequest::builder()
            .method(zed_extension_api::http_client::HttpMethod::Get)
            .url(&url)
            .build()?;
        let nuget_package_response = zed::http_client::fetch(&request)?;

        let nuget_packages: NuGetPackagesResponse =
            serde_json::from_slice(&nuget_package_response.body.as_slice())
                .map_err(|e| e.to_string())?;

        let package = nuget_packages
            .value
            .iter()
            .flat_map(|p| p.versions.iter())
            .find(|v| v.is_latest == true)
            .unwrap();

        Ok(package.version.clone())
    }

    // fn make_executable(path: &str) -> Result<()> {
    // }
}
