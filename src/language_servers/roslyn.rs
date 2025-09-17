use std::fs;
use zed_extension_api::{
    self as zed, serde_json::Map, settings::LspSettings, LanguageServerId, Result,
};

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
                args: binary_args.unwrap_or_default(),
                env: Default::default(),
            });
        }

        todo!("Automatic installation of the roslyn language server is not yet implemented. Please specify the path to the language server binary in the settings.");
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