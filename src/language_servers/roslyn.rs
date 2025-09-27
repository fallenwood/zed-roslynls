use std::fs;
use zed_extension_api::{
    self as zed, LanguageServerId, Result, serde_json::Map, settings::LspSettings,
};

const ORGANIZATION: &str = "azure-public";
const PROJECT: &str = "vside";
const FEED: &str = "vs-impl";

// TODO: Check update instead of hard encoded version
const PACKAGE_VERSION: &str = "5.1.0-1.25476.5";

pub struct Roslyn {
    cached_binary_path: Option<String>,
}

impl Roslyn {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn";

    pub fn new() -> Self {
        Roslyn {
            cached_binary_path: None,
        }
    }

    pub fn language_server_cmd(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let default_args: Vec<String> = vec![
            "--logLevel".into(),
            "Information".into(),
            "--extensionLogDirectory".into(),
            ".roslynls".into(),
            "--stdio".into(),
        ];

        let binary_settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
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
            return Ok(zed::Command {
                command: path,
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

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

        let executable = match platform {
            zed_extension_api::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };

        if let Some(path) = worktree.which(executable) {
            return Ok(zed::Command {
                command: path,
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

        if let Some(path) = &self.cached_binary_path
            && fs::metadata(path).map_or(false, |stat| stat.is_file())
        {
            return Ok(zed::Command {
                command: path.clone(),
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

        let package_id = format!("Microsoft.CodeAnalysis.LanguageServer.{runtime_identifier}");
        let asset_name = format!(
            "{package_id}.{version}.{extension}",
            package_id = package_id.clone(),
            version = PACKAGE_VERSION,
            extension = "nupkg",
        );

        let url = format!(
            "https://pkgs.dev.azure.com/{ORGANIZATION}/{PROJECT}/_packaging/{FEED}/nuget/v3/flat2/{package_id}/{PACKAGE_VERSION}/{asset_name}"
        );

        let version_dir = format!(
            "{package_id}-{version}",
            package_id = package_id,
            version = PACKAGE_VERSION,
        );

        let binary_path =
            format!("{version_dir}/content/LanguageServer/{runtime_identifier}/{executable}");

        if fs::metadata(binary_path.clone()).map_or(false, |stat| stat.is_file()) {
            self.cached_binary_path = Some(binary_path.clone());

            return Ok(zed::Command {
                command: binary_path.clone(),
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

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

        return Ok(zed::Command {
            command: binary_path,
            args: binary_args.unwrap_or(default_args),
            env: Default::default(),
        });
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
}
