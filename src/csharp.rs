mod language_servers;

use language_servers::Roslyn;
use zed_extension_api::{self as zed, Result};

struct CsharpExtension {
    roslyn: Option<Roslyn>,
}

impl CsharpExtension {}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            roslyn: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            Roslyn::LANGUAGE_SERVER_ID => {
                let roslyn = self.roslyn.get_or_insert_with(Roslyn::new);
                roslyn.language_server_cmd(language_server_id, worktree)
            }
            language_server_id => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        if language_server_id.as_ref() == Roslyn::LANGUAGE_SERVER_ID {
            if let Some(roslyn) = self.roslyn.as_mut() {
                return roslyn.configuration_options(worktree);
            }
        }
        Ok(None)
    }
}

zed::register_extension!(CsharpExtension);