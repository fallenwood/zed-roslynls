use serde_json::Value;
use std::
    fs::{self}
;
use zed_extension_api::{
    self as zed, Result,
    settings::LspSettings,
};

use crate::utils;

const NETCOREDBG_REPO: &str = "marcptrs/netcoredbg";
const NETCOREDBG_TAG: &str = "v3.1.2-1054";
const NETCOREDBG: &str = "netcoredbg";

pub struct NetcoreDbg {
    cached_netcoredbg_path: Option<String>,
}

impl NetcoreDbg {
    pub const DEBUG_ADAPTER_ID: &'static str = "netcoredbg";

    pub fn new() -> Self {
        NetcoreDbg {
            cached_netcoredbg_path: None,
        }
    }

    pub fn get_dap_binary(
        &mut self,
        _adapter_name: String,
        config: zed::DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<zed::DebugAdapterBinary, String> {
        let workspace_folder = worktree.root_path();

        let command = self.ensure_netcoredbg(worktree)?;

        let mut raw_json: Value = zed::serde_json::from_str(&config.config)
            .map_err(|e| format!("Failed to parse debug configuration: {e}"))?;
        let mut config_json = if let Some(inner) = raw_json.get_mut("config") {
            inner.take()
        } else {
            raw_json
        };

        if let Some(obj) = config_json.as_object_mut() {
            for (_key, value) in obj.iter_mut() {
                if let Some(s) = value.as_str() {
                    let expanded = s.replace("${workspaceFolder}", &workspace_folder);
                    *value = Value::String(expanded);
                }
            }
        }

        let request_kind = match config_json.get("request") {
            Some(launch) if launch == "launch" => {
                zed::StartDebuggingRequestArgumentsRequest::Launch
            }
            Some(attach) if attach == "attach" => {
                zed::StartDebuggingRequestArgumentsRequest::Attach
            }
            _ => zed::StartDebuggingRequestArgumentsRequest::Launch,
        };

        let config_str = zed::serde_json::to_string(&config_json)
            .map_err(|e| format!("Failed to serialize debug configuration: {e}"))?;

        Ok(zed::DebugAdapterBinary {
            command: Some(command.command),
            arguments: command.args,
            cwd: Some(worktree.root_path()),
            envs: command.env,
            request_args: zed::StartDebuggingRequestArguments {
                request: request_kind,
                configuration: config_str,
            },
            connection: None,
        })
    }

    pub fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        config: serde_json::Value,
    ) -> Result<zed::StartDebuggingRequestArgumentsRequest, String> {
        if config.is_null() {
            return Err("Config is null - awaiting locator resolution".to_string());
        }

        let cfg = if let Some(inner) = config.get("config") {
            inner
        } else {
            &config
        };
        match cfg.get("request") {
            Some(launch) if launch == "launch" => {
                Ok(zed::StartDebuggingRequestArgumentsRequest::Launch)
            }
            Some(attach) if attach == "attach" => {
                Ok(zed::StartDebuggingRequestArgumentsRequest::Attach)
            }
            Some(value) => Err(format!(
                "Unexpected value for `request` key in C# debug adapter configuration: {value:?}"
            )),
            None => Err("Missing `request` field in debug configuration".to_string()),
        }
    }

    fn ensure_netcoredbg(
        self: &mut Self,
        worktree: &zed::Worktree,
    ) -> std::result::Result<zed::Command, String> {
        let default_args = vec!["--interpreter=vscode".to_string()];

        let settings = LspSettings::for_worktree(Self::DEBUG_ADAPTER_ID, worktree).ok();
        let binary_settings = settings.and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        if let Some(path) = binary_settings
            .and_then(|binary_settings| binary_settings.path)
            .or_else(|| {
                self.cached_netcoredbg_path
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

        let executable = utils::get_executable(NETCOREDBG);

        if let Some(path) = worktree.which(executable.as_str()) {
            return Ok(zed::Command {
                command: path,
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

        if let Some(path) = &self.cached_netcoredbg_path
            && fs::metadata(path).map_or(false, |stat| stat.is_file())
        {
            return Ok(zed::Command {
                command: path.into(),
                args: binary_args.unwrap_or(default_args),
                env: Default::default(),
            });
        }

        let netcoredbg_path = utils::ensure_github_release(
            NETCOREDBG,
            NETCOREDBG_REPO,
            NETCOREDBG_TAG,
            || Self::get_netcoredbg_package_id(),
            Self::get_github_asset_file_type())?;

        return Ok(zed::Command {
            command: netcoredbg_path,
            args: binary_args.unwrap_or(default_args),
            env: Default::default(),
        });
    }

    fn get_netcoredbg_package_id() -> String {
        let runtime_identifier = utils::get_runtime_identifier();
        let (os, _) = zed::current_platform();

        let ext = match os {
            zed::Os::Windows => "zip",
            _ => "tar.gz",
        };

        format!("{NETCOREDBG}-{runtime_identifier}.{ext}")
    }

    fn get_github_asset_file_type() -> zed::DownloadedFileType {
        let (os, _) = zed::current_platform();
        match os {
            zed::Os::Windows => zed_extension_api::DownloadedFileType::Zip,
            _ => zed::DownloadedFileType::GzipTar,
        }
    }
}
