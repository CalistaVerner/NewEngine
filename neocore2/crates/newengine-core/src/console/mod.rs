#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString};
use newengine_plugin_api::{Blob, CapabilityId, MethodName, ServiceV1, ServiceV1Dyn};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use crate::host_services;
use crate::plugins::{host_api, host_context};

pub const COMMAND_SERVICE_ID: &str = "engine.command";

pub mod method {
    pub const EXEC: &str = "command.exec";
    pub const LIST: &str = "command.list";
    pub const COMPLETE: &str = "command.complete";
    pub const REFRESH: &str = "command.refresh";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayloadMode {
    /// Use the raw tail after the command token as UTF-8 bytes.
    RawTailUtf8,
    /// No payload.
    Empty,
}

#[derive(Debug, Clone)]
enum DynAction {
    /// Call an existing ServiceV1 method.
    ServiceCall {
        service_id: String,
        method: String,
        payload: PayloadMode,
    },
    /// Expand into another command line (alias/macro).
    Alias { expand: String },
}

#[derive(Debug, Clone)]
struct DynCommand {
    help: String,
    action: DynAction,
    source: String,
}

type StaticCommandFn =
fn(rt: &ConsoleRuntime, args: &[&str], raw: &str) -> Result<String, String>;

struct StaticCommandEntry {
    help: &'static str,
    f: StaticCommandFn,
}

struct StaticRegistry {
    map: BTreeMap<&'static str, StaticCommandEntry>,
}

impl StaticRegistry {
    fn new() -> Self {
        Self { map: BTreeMap::new() }
    }

    fn register(&mut self, name: &'static str, help: &'static str, f: StaticCommandFn) {
        self.map.insert(name, StaticCommandEntry { help, f });
    }

    fn exec(
        &self,
        rt: &ConsoleRuntime,
        cmd: &str,
        args: &[&str],
        raw: &str,
    ) -> Result<String, String> {
        let e = self
            .map
            .get(cmd)
            .ok_or_else(|| format!("unknown command: {cmd} (try: help)"))?;
        (e.f)(rt, args, raw)
    }

    fn list_lines(&self) -> Vec<String> {
        self.map
            .iter()
            .map(|(k, v)| format!("{k} - {}", v.help))
            .collect()
    }

    fn complete(&self, prefix: &str) -> Vec<String> {
        let p = prefix.trim();
        if p.is_empty() {
            return self.map.keys().map(|s| s.to_string()).collect();
        }
        self.map
            .keys()
            .filter(|k| k.starts_with(p))
            .map(|s| s.to_string())
            .collect()
    }

    fn contains(&self, cmd: &str) -> bool {
        self.map.contains_key(cmd)
    }
}

/// Runtime state for console/commands.
/// Lives in engine-core and cannot be removed.
pub struct ConsoleRuntime {
    static_reg: StaticRegistry,
    exit_requested: AtomicBool,

    history: Mutex<VecDeque<String>>,
    output: Mutex<VecDeque<String>>,

    dyn_state: Mutex<DynState>,
}

struct DynState {
    last_services_gen: u64,
    dyn_cmds: HashMap<String, DynCommand>,
}

impl ConsoleRuntime {
    fn new() -> Self {
        let mut reg = StaticRegistry::new();

        reg.register("help", "List available commands", |rt, _args, _raw| {
            rt.refresh_dynamic_commands_if_needed();

            let mut lines = Vec::new();
            lines.push("=== builtin ===".to_string());
            lines.extend(rt.static_reg.list_lines());

            let dyn_lines = rt.list_dynamic_lines();
            if !dyn_lines.is_empty() {
                lines.push("=== plugins ===".to_string());
                lines.extend(dyn_lines);
            }

            Ok(lines.join("\n"))
        });

        reg.register("services", "List registered ServiceV1 ids", |_rt, _args, _raw| {
            let ids = host_services::list_service_ids();
            Ok(ids.join("\n"))
        });

        reg.register(
            "describe",
            "Print ServiceV1 describe() json/text: describe <service_id>",
            |_rt, args, _raw| {
                if args.len() < 2 {
                    return Err("usage: describe <service_id>".to_string());
                }
                let sid = args[1];
                let d = host_services::describe_service(sid)
                    .ok_or_else(|| format!("service not found: {sid}"))?;
                Ok(d)
            },
        );

        reg.register(
            "call",
            "Call ServiceV1: call <service_id> <method> <payload_utf8>",
            |_rt, args, raw| {
                if args.len() < 4 {
                    return Err("usage: call <service_id> <method> <payload_utf8>".to_string());
                }
                let service_id = args[1];
                let method = args[2];

                let payload = raw
                    .splitn(4, ' ')
                    .nth(3)
                    .unwrap_or("")
                    .as_bytes()
                    .to_vec();

                let out = host_services::call_service_v1(service_id, method, &payload)?;
                Ok(String::from_utf8_lossy(&out).to_string())
            },
        );

        reg.register("refresh", "Force refresh plugin commands", |rt, _args, _raw| {
            rt.refresh_dynamic_commands(true);
            Ok("refreshed".to_string())
        });

        reg.register("quit", "Request engine exit", |rt, _args, _raw| {
            rt.exit_requested.store(true, Ordering::Release);
            Ok("exit requested".to_string())
        });

        Self {
            static_reg: reg,
            exit_requested: AtomicBool::new(false),
            history: Mutex::new(VecDeque::with_capacity(256)),
            output: Mutex::new(VecDeque::with_capacity(2048)),
            dyn_state: Mutex::new(DynState {
                last_services_gen: 0,
                dyn_cmds: HashMap::new(),
            }),
        }
    }

    #[inline]
    pub fn take_exit_requested(&self) -> bool {
        self.exit_requested.swap(false, Ordering::AcqRel)
    }

    fn push_history(&self, line: String) {
        let mut h = self.history.lock();
        if h.len() >= 256 {
            h.pop_front();
        }
        h.push_back(line);
    }

    fn push_output_line(&self, line: String) {
        let mut o = self.output.lock();
        if o.len() >= 2048 {
            o.pop_front();
        }
        o.push_back(line);
    }

    pub fn drain_output(&self) -> Vec<String> {
        let mut o = self.output.lock();
        o.drain(..).collect()
    }

    fn list_dynamic_lines(&self) -> Vec<String> {
        let st = self.dyn_state.lock();
        let mut out: Vec<String> = st
            .dyn_cmds
            .iter()
            .map(|(k, v)| format!("{k} - {} ({})", v.help, v.source))
            .collect();
        out.sort();
        out
    }

    fn refresh_dynamic_commands_if_needed(&self) {
        let gen = host_context::services_generation();
        let need = {
            let st = self.dyn_state.lock();
            st.last_services_gen != gen
        };
        if need {
            self.refresh_dynamic_commands(false);
        }
    }

    fn refresh_dynamic_commands(&self, force: bool) {
        let gen = host_context::services_generation();
        let mut st = self.dyn_state.lock();
        if !force && st.last_services_gen == gen {
            return;
        }

        let snapshot = {
            let c = host_context::ctx();
            let g = match c.services.lock() {
                Ok(v) => v,
                Err(_) => {
                    st.last_services_gen = gen;
                    st.dyn_cmds.clear();
                    return;
                }
            };

            g.iter()
                .map(|(id, svc)| (id.clone(), svc.describe().to_string()))
                .collect::<Vec<(String, String)>>()
        };

        let mut dyn_cmds: HashMap<String, DynCommand> = HashMap::new();

        for (service_id, desc) in snapshot {
            if let Some(cmds) = parse_console_commands(&service_id, &desc) {
                for c in cmds {
                    dyn_cmds.insert(c.0, c.1);
                }
            }
        }

        st.dyn_cmds = dyn_cmds;
        st.last_services_gen = gen;
    }

    fn exec_line(&self, line: &str) -> CommandExecResponse {
        self.refresh_dynamic_commands_if_needed();

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return CommandExecResponse {
                ok: true,
                output: String::new(),
                error: String::new(),
            };
        }

        self.push_history(trimmed.to_string());

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = tokens[0];

        if self.static_reg.contains(cmd) {
            match self.static_reg.exec(self, cmd, &tokens, trimmed) {
                Ok(out) => {
                    if !out.is_empty() {
                        for l in out.lines() {
                            self.push_output_line(l.to_string());
                        }
                    }
                    return CommandExecResponse {
                        ok: true,
                        output: out,
                        error: String::new(),
                    };
                }
                Err(err) => {
                    self.push_output_line(format!("ERR: {err}"));
                    return CommandExecResponse {
                        ok: false,
                        output: String::new(),
                        error: err,
                    };
                }
            }
        }

        match self.exec_dynamic(cmd, trimmed) {
            Ok(out) => {
                if !out.is_empty() {
                    for l in out.lines() {
                        self.push_output_line(l.to_string());
                    }
                }
                CommandExecResponse {
                    ok: true,
                    output: out,
                    error: String::new(),
                }
            }
            Err(err) => {
                self.push_output_line(format!("ERR: {err}"));
                CommandExecResponse {
                    ok: false,
                    output: String::new(),
                    error: err,
                }
            }
        }
    }

    fn exec_dynamic(&self, cmd: &str, raw: &str) -> Result<String, String> {
        let action = {
            let st = self.dyn_state.lock();
            let dc = st
                .dyn_cmds
                .get(cmd)
                .ok_or_else(|| format!("unknown command: {cmd} (try: help)"))?;
            dc.action.clone()
        };

        match action {
            DynAction::Alias { expand } => {
                // Alias expands into: "<expand> <tail>"
                let tail = raw.splitn(2, ' ').nth(1).unwrap_or("").trim();
                let expanded = if tail.is_empty() {
                    expand
                } else {
                    format!("{expand} {tail}")
                };
                let resp = self.exec_line(&expanded);
                if resp.ok {
                    Ok(resp.output)
                } else {
                    Err(resp.error)
                }
            }
            DynAction::ServiceCall {
                service_id,
                method,
                payload,
            } => {
                let payload_bytes = match payload {
                    PayloadMode::Empty => Vec::new(),
                    PayloadMode::RawTailUtf8 => raw
                        .splitn(2, ' ')
                        .nth(1)
                        .unwrap_or("")
                        .as_bytes()
                        .to_vec(),
                };

                let out = host_services::call_service_v1(&service_id, &method, &payload_bytes)?;
                Ok(String::from_utf8_lossy(&out).to_string())
            }
        }
    }

    fn complete(&self, prefix: &str) -> Vec<String> {
        self.refresh_dynamic_commands_if_needed();

        let mut out = self.static_reg.complete(prefix);

        let st = self.dyn_state.lock();
        let p = prefix.trim();
        if p.is_empty() {
            out.extend(st.dyn_cmds.keys().cloned());
        } else {
            out.extend(
                st.dyn_cmds
                    .keys()
                    .filter(|k| k.starts_with(p))
                    .cloned(),
            );
        }

        out.sort();
        out.dedup();
        out
    }
}

fn parse_console_commands(service_id: &str, describe_json: &str) -> Option<Vec<(String, DynCommand)>> {
    let v: Value = serde_json::from_str(describe_json).ok()?;
    let console = v.get("console")?;
    let commands = console.get("commands")?.as_array()?;

    let mut out = Vec::new();

    for item in commands {
        let name = item.get("name")?.as_str()?.trim();
        if name.is_empty() {
            continue;
        }

        let help = item
            .get("help")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let kind = item.get("kind").and_then(|x| x.as_str()).unwrap_or("service_call");

        let dc = match kind {
            "alias" => {
                let expand = item.get("expand")?.as_str()?.to_string();
                DynCommand {
                    help: if help.is_empty() {
                        format!("alias -> {expand}")
                    } else {
                        help
                    },
                    action: DynAction::Alias { expand },
                    source: service_id.to_string(),
                }
            }
            _ => {
                let target_service = item
                    .get("service_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or(service_id)
                    .to_string();

                let method = item.get("method")?.as_str()?.to_string();

                let payload = match item.get("payload").and_then(|x| x.as_str()) {
                    Some("empty") => PayloadMode::Empty,
                    _ => PayloadMode::RawTailUtf8,
                };

                DynCommand {
                    help: if help.is_empty() {
                        format!("service call: {target_service}::{method}")
                    } else {
                        help
                    },
                    action: DynAction::ServiceCall {
                        service_id: target_service,
                        method,
                        payload,
                    },
                    source: service_id.to_string(),
                }
            }
        };

        out.push((name.to_string(), dc));
    }

    Some(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandExecResponse {
    pub ok: bool,
    pub output: String,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandListResponse {
    pub commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandCompleteResponse {
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandRefreshResponse {
    pub ok: bool,
}

struct CommandService {
    rt: Arc<ConsoleRuntime>,
}

impl CommandService {
    fn new(rt: Arc<ConsoleRuntime>) -> Self {
        Self { rt }
    }
}

impl ServiceV1 for CommandService {
    fn id(&self) -> CapabilityId {
        RString::from(COMMAND_SERVICE_ID)
    }

    fn describe(&self) -> RString {
        let d = json!({
            "id": COMMAND_SERVICE_ID,
            "version": 2,
            "methods": [
                { "name": method::EXEC, "payload": "utf8 line", "returns": "json CommandExecResponse" },
                { "name": method::LIST, "payload": "empty", "returns": "json CommandListResponse" },
                { "name": method::COMPLETE, "payload": "utf8 prefix", "returns": "json CommandCompleteResponse" },
                { "name": method::REFRESH, "payload": "empty", "returns": "json CommandRefreshResponse" }
            ]
        });
        RString::from(d.to_string())
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        let m = method.to_string();
        match m.as_str() {
            method::EXEC => {
                let line = String::from_utf8_lossy(payload.as_slice()).to_string();
                let resp = self.rt.exec_line(&line);
                let bytes = serde_json::to_vec(&resp).unwrap_or_else(|e| {
                    format!("{{\"ok\":false,\"output\":\"\",\"error\":\"{e}\"}}").into_bytes()
                });
                RResult::ROk(Blob::from(bytes))
            }
            method::LIST => {
                self.rt.refresh_dynamic_commands_if_needed();
                let mut cmds = self.rt.static_reg.complete("");
                {
                    let st = self.rt.dyn_state.lock();
                    cmds.extend(st.dyn_cmds.keys().cloned());
                }
                cmds.sort();
                cmds.dedup();

                let resp = CommandListResponse { commands: cmds };
                let bytes = serde_json::to_vec(&resp).unwrap_or_default();
                RResult::ROk(Blob::from(bytes))
            }
            method::COMPLETE => {
                let prefix = String::from_utf8_lossy(payload.as_slice()).to_string();
                let resp = CommandCompleteResponse {
                    items: self.rt.complete(&prefix),
                };
                let bytes = serde_json::to_vec(&resp).unwrap_or_default();
                RResult::ROk(Blob::from(bytes))
            }
            method::REFRESH => {
                self.rt.refresh_dynamic_commands(true);
                let resp = CommandRefreshResponse { ok: true };
                let bytes = serde_json::to_vec(&resp).unwrap_or_default();
                RResult::ROk(Blob::from(bytes))
            }
            _ => RResult::RErr(RString::from(format!("unknown method: {m}"))),
        }
    }
}

static CONSOLE_RT: OnceLock<Arc<ConsoleRuntime>> = OnceLock::new();

pub(crate) fn init_console_service() {
    let rt = CONSOLE_RT.get_or_init(|| Arc::new(ConsoleRuntime::new())).clone();

    let svc = CommandService::new(rt);

    let dyn_svc: ServiceV1Dyn<'static> =
        ServiceV1Dyn::from_value(svc, abi_stable::sabi_trait::TD_Opaque);

    let _ = host_api::host_register_service_impl(dyn_svc, false);
}

#[inline]
pub fn take_exit_requested() -> bool {
    CONSOLE_RT
        .get()
        .map(|rt| rt.take_exit_requested())
        .unwrap_or(false)
}

#[inline]
pub fn drain_output() -> Vec<String> {
    CONSOLE_RT.get().map(|rt| rt.drain_output()).unwrap_or_default()
}