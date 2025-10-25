mod debug_adapters;
mod language_servers;
mod utils;

use language_servers::Roslyn;
use zed_extension_api::{self as zed, Result};

use crate::debug_adapters::NetcoreDbg;

struct CsharpExtension {
    roslyn: Option<Roslyn>,
    netcoredbg: Option<NetcoreDbg>,
}

impl CsharpExtension {}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            roslyn: None,
            netcoredbg: None,
        }
    }

    // Language Server Support
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

    // Debug Adapter Support
    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<zed_extension_api::DebugAdapterBinary, String> {
        if adapter_name == NetcoreDbg::DEBUG_ADAPTER_ID {
            let netcoredbg = self.netcoredbg.get_or_insert_with(NetcoreDbg::new);
            return netcoredbg.get_dap_binary(
                adapter_name,
                config,
                user_provided_debug_adapter_path,
                worktree,
            );
        }

        Err(format!("unknown debug adapter: {adapter_name}"))
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> Result<zed_extension_api::StartDebuggingRequestArgumentsRequest, String> {
        if adapter_name == NetcoreDbg::DEBUG_ADAPTER_ID {
            let netcoredbg = self.netcoredbg.get_or_insert_with(NetcoreDbg::new);
            return netcoredbg.dap_request_kind(adapter_name, config);
        }

        Err(format!("unknown debug adapter: {adapter_name}"))
    }

    fn dap_config_to_scenario(
        &mut self,
        config: zed_extension_api::DebugConfig,
    ) -> Result<zed_extension_api::DebugScenario, String> {
        let netcoredbg = self.netcoredbg.get_or_insert_with(NetcoreDbg::new);
        return netcoredbg.dap_config_to_scenario(config);
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: zed_extension_api::TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<zed_extension_api::DebugScenario> {
        if locator_name == NetcoreDbg::DEBUG_ADAPTER_ID {
            let netcoredbg = self.netcoredbg.get_or_insert_with(NetcoreDbg::new);
            return netcoredbg.dap_locator_create_scenario(
                locator_name,
                build_task,
                resolved_label,
                debug_adapter_name,
            );
        }

        None
    }
}

zed::register_extension!(CsharpExtension);
