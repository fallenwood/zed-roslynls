use serde_json::Value;
use std::{
    fs::{self, FileType},
    path::{Path, PathBuf},
};
use zed_extension_api::{
    self as zed, LanguageServerId, Result, serde_json::Map, settings::LspSettings,
};

const NETCOREDBG_REPO: &str = "marcptrs/netcoredbg";
const NETCOREDBG_TAG: &str = "v3.1.2-1054";

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
        config: zed_extension_api::DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<zed_extension_api::DebugAdapterBinary, String> {
        let workspace_folder = worktree.root_path();

        let command = zed::Command {
            command: "/home/vbox/.local/opt/netcoredbg/netcoredbg".to_string(),
            args: vec!["--interpreter=vscode".to_string()],
            env: Default::default(),
        };

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
    ) -> Result<zed_extension_api::StartDebuggingRequestArgumentsRequest, String> {
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

    pub fn dap_config_to_scenario(
        &mut self,
        config: zed::DebugConfig,
    ) -> Result<zed::DebugScenario, String> {
        let (program, cwd, args, envs) = match config.request {
            zed::DebugRequest::Launch(ref launch) => {
                let program = launch.program.clone();
                let cwd = launch.cwd.clone().unwrap_or_else(|| ".".to_string());
                let args = launch.args.clone();
                let envs = launch.envs.clone();
                (program, cwd, args, envs)
            }
            zed::DebugRequest::Attach(_) => {
                return Err("Attach is not supported via dap_config_to_scenario".to_string());
            }
        };

        let mut debug_config = serde_json::Map::new();
        debug_config.insert("type".to_string(), Value::String("netcoredbg".to_string()));
        debug_config.insert("request".to_string(), Value::String("launch".to_string()));
        debug_config.insert("program".to_string(), Value::String(program.clone()));
        debug_config.insert("cwd".to_string(), Value::String(cwd.clone()));

        if !args.is_empty() {
            debug_config.insert(
                "args".to_string(),
                Value::Array(args.iter().map(|a| Value::String(a.clone())).collect()),
            );
        }

        if !envs.is_empty() {
            let env_obj: serde_json::Map<String, Value> = envs
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            debug_config.insert("env".to_string(), Value::Object(env_obj));
        }

        let stop_at_entry = config.stop_on_entry.unwrap_or(false);
        debug_config.insert("stopAtEntry".to_string(), Value::Bool(stop_at_entry));
        debug_config.insert(
            "console".to_string(),
            Value::String("integratedTerminal".to_string()),
        );

        let config_str = zed::serde_json::to_string(&debug_config)
            .map_err(|e| format!("Failed to serialize debug configuration: {e}"))?;

        Ok(zed::DebugScenario {
            label: format!(
                "Debug {}",
                program.split('/').next_back().unwrap_or(&program)
            ),
            adapter: config.adapter,
            build: None,
            config: config_str,
            tcp_connection: None,
        })
    }

    pub fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: zed::TaskTemplate,
        resolved_label: String,
        _debug_adapter_name: String,
    ) -> Option<zed::DebugScenario> {
        let cmd = &build_task.command;
        {
            let cmd_name = Path::new(cmd)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(cmd);
            let is_dotnet = cmd_name == "dotnet" || cmd_name == "dotnet.exe";
            if !is_dotnet {
                return None;
            }
        }

        let collect_program_args = |args: &Vec<String>| -> Vec<String> {
            if let Some(idx) = args.iter().position(|a| a == "--") {
                args[idx + 1..].to_vec()
            } else {
                Vec::new()
            }
        };

        let args = build_task.args.clone();
        if args.is_empty() {
            return None;
        }

        let program_args = collect_program_args(&args);

        let derived_build_task = match args.first().map(|s| s.as_str()) {
            Some("run") => {
                let mut derived = build_task.clone();
                let mut new_args = vec!["build".to_string()];

                let cwd = build_task.cwd.as_ref().map(|s| s.as_str()).unwrap_or(".");

                let mut iter = args.iter().skip(1);
                while let Some(arg) = iter.next() {
                    if arg == "--" {
                        break;
                    } else if arg == "--project" {
                        if let Some(project_file) = iter.next() {
                            let project_path =
                                if project_file.starts_with('/') || project_file.contains(":\\") {
                                    project_file.clone()
                                } else {
                                    let mut full_path = PathBuf::from(cwd);
                                    full_path.push(project_file);
                                    full_path.to_string_lossy().to_string()
                                };
                            new_args.push(project_path);
                        }
                    } else if !arg.starts_with("--") || arg == "--configuration" || arg == "-c" {
                        new_args.push(arg.clone());
                        if arg == "--configuration" || arg == "-c" {
                            if let Some(val) = iter.next() {
                                new_args.push(val.clone());
                            }
                        }
                    }
                }

                derived.args = new_args;
                derived
            }
            _ => {
                return None;
            }
        };

        let mut derived_build_task = derived_build_task;
        let mut env = derived_build_task.env.clone();
        if !program_args.is_empty() {
            env.push((
                "ZED_DOTNET_PROGRAM_ARGS".to_string(),
                serde_json::to_string(&program_args).unwrap_or_default(),
            ));
        }
        derived_build_task.env = env;

        Some(zed::DebugScenario {
            label: format!("Debug {}", resolved_label),
            adapter: "netcoredbg".to_string(),
            build: Some(zed::BuildTaskDefinition::Template(
                zed::BuildTaskDefinitionTemplatePayload {
                    template: derived_build_task.clone(),
                    locator_name: Some(locator_name.clone()),
                },
            )),
            config: "null".to_string(),
            tcp_connection: None,
        })
    }
}
