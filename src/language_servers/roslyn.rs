use std::fs;
use zed_extension_api::{
    self as zed, LanguageServerId, Result, serde_json::Map, settings::LspSettings,
};

use crate::language_servers::model::NuGetPackagesResponse;

const ORGANIZATION: &str = "azure-public";
const PROJECT: &str = "vside";
const FEED: &str = "vs-impl";
const WRAPPER_PATH_KEY: &str = "wrapper_path";

// Example version
// const PACKAGE_VERSION: &str = "5.1.0-1.25476.5";

pub struct Roslyn {
    cached_binary_path: Option<String>,
    cached_wrapper_path: Option<String>,
}

impl Roslyn {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn";

    pub fn new() -> Self {
        Roslyn {
            cached_binary_path: None,
            cached_wrapper_path: None,
        }
    }

    pub fn language_server_cmd(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (platform, arch) = zed::current_platform();

        let runtime_identifier = format!(
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
        );

        let package_id = format!("Microsoft.CodeAnalysis.LanguageServer.{runtime_identifier}");

        // TODO: use configured wrapper path
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok();

        let wrapper_path = settings
            .as_ref()
            .and_then(|lsp_settings| {
                lsp_settings.settings.as_ref()
            })
            .and_then(|lsp_settings| {
                if let zed::serde_json::Value::Object(settings_map) = lsp_settings {
                    settings_map.get(WRAPPER_PATH_KEY).and_then(|value| {
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

        let wrapper_path = if let Some(path) = wrapper_path {
            path
        } else if let Some(cached_path) = &self.cached_wrapper_path {
            cached_path.clone()
        } else {
            println!("No roslynls wrapper found");
            // TODO: download roslynls wrapper
            "roslynls".into()
        };

        self.cached_wrapper_path = Some(wrapper_path.clone());

        let binary_settings = settings
            .and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        if let Some(path) = binary_settings
            .and_then(|binary_settings| binary_settings.path)
            .or_else(|| {
                self.cached_binary_path
                    .as_ref()
                    .filter(|path| fs::metadata(path).map_or(false, |stat| stat.is_file()))
                    .cloned()
            })
        {
            return Self::cmd(wrapper_path, path, worktree.root_path().to_string(), binary_args)
        }

        // Fetch latest version
        let version = {
            let url = format!(
                "https://feeds.dev.azure.com/{ORGANIZATION}/{PROJECT}/_apis/packaging/feeds/{FEED}/packages?packageNameQuery={package_id}&api-version=6.0-preview.1",
            );

            println!(
                "Fetching latest Roslyn Language Server version from: {}",
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

            let version = package.version.clone();

            version
        };

        let executable = match platform {
            zed_extension_api::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };

        if let Some(path) = worktree.which(executable) {
            return Self::cmd(wrapper_path, path, worktree.root_path().to_string(), binary_args)
        }

        if let Some(path) = &self.cached_binary_path
            && fs::metadata(path).map_or(false, |stat| stat.is_file())
        {
            return Self::cmd(wrapper_path, path.clone(), worktree.root_path().to_string(), binary_args)
        }

        let asset_name = format!(
            "{package_id}.{version}.{extension}",
            package_id = package_id.clone(),
            version = version,
            extension = "nupkg",
        );

        let version_dir = format!(
            "{package_id}-{version}",
            package_id = package_id,
            version = version,
        );

        let current_dir = std::env::current_dir().unwrap().to_str().unwrap().to_string();

        let binary_path =
            format!("{current_dir}/{version_dir}/content/LanguageServer/{runtime_identifier}/{executable}");

        if fs::metadata(binary_path.clone()).map_or(false, |stat| stat.is_file()) {
            self.cached_binary_path = Some(binary_path.clone());

            return Self::cmd(wrapper_path, binary_path.clone(), worktree.root_path().to_string(), binary_args)
        }

        let url = format!(
            "https://pkgs.dev.azure.com/{ORGANIZATION}/{PROJECT}/_packaging/{FEED}/nuget/v3/flat2/{package_id}/{version}/{asset_name}"
        );

        println!("Downloading Roslyn Language Server from: {}", url.clone());

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

        self.cached_binary_path = Some(binary_path.clone());

        Self::cmd(wrapper_path, binary_path, worktree.root_path().to_string(), binary_args)
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
                if key == WRAPPER_PATH_KEY {
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

    fn cmd(wrapper_path: String, lsp_path: String, project_root: String, binary_args: Option<Vec<String>>) -> Result<zed::Command> {
        let default_args: Vec<String> = vec![
            "--lsp".into(),
            lsp_path,
            "--project-root".into(),
            project_root
        ];

        return Ok(zed::Command {
            command: wrapper_path,
            args: binary_args.unwrap_or(default_args),
            env: Default::default(),
        });
    }
}
