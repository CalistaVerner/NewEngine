#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString};
use newengine_plugin_api::{Blob, CapabilityId, MethodName, ServiceV1, ServiceV1Dyn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use crate::plugins::host_api;
use crate::plugins::host_context;

pub const COMMAND_SERVICE_ID: &str = "engine.command";

pub mod method {
    pub const EXEC: &str = "command.exec";
    pub const COMPLETE: &str = "command.complete";
    pub const SUGGEST: &str = "command.suggest";
    pub const REFRESH: &str = "command.refresh";
}

type CmdFn = fn(&ConsoleRuntime, &str) -> Result<String, String>;

struct Cmd {
    help: &'static str,
    usage: &'static str,
    f: CmdFn,
}

#[derive(Debug, Clone, Deserialize)]
struct ConsoleCmdEntry {
    name: String,
    #[serde(default)]
    help: Option<String>,
    #[serde(default)]
    usage: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    service_id: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    payload: Option<String>,
}

#[derive(Debug, Clone)]
struct DynCommand {
    help: String,
    usage: String,
    service_id: String,
    method: String,
    payload: DynPayload,
}

#[derive(Debug, Clone, Copy)]
enum DynPayload {
    Empty,
    Raw,
}

#[derive(Debug, Clone, Serialize)]
struct SuggestItem {
    kind: String,
    display: String,
    insert: String,
    help: String,
    usage: String,
}

#[derive(Debug, Clone, Serialize)]
struct SuggestResponse {
    signature: String,
    items: Vec<SuggestItem>,
}


pub struct ConsoleRuntime {
    cmds: BTreeMap<&'static str, Cmd>,
    dyn_cmds: std::sync::Mutex<BTreeMap<String, DynCommand>>,
    method_cache: std::sync::Mutex<BTreeMap<String, Vec<String>>>,
    exit_requested: AtomicBool,
}

impl ConsoleRuntime {
    fn new() -> Self {
        let mut cmds = BTreeMap::new();

        cmds.insert(
            "help",
            Cmd {
                help: "List commands",
                usage: "help",
                f: |rt, _| rt.help_text(),
            },
        );

        cmds.insert(
            "services",
            Cmd {
                help: "List services",
                usage: "services",
                f: |_, _| {
                    let c = host_context::ctx();
                    let g = c.services.lock().unwrap();
                    Ok(g.keys().cloned().collect::<Vec<_>>().join("\n"))
                },
            },
        );

        cmds.insert(
            "refresh",
            Cmd {
                help: "Refresh console commands from services",
                usage: "refresh",
                f: |rt, _| {
                    rt.refresh_dyn_commands();
                    Ok("refreshed".into())
                },
            },
        );

        cmds.insert(
            "describe",
            Cmd {
                help: "Describe a service",
                usage: "describe <service_id>",
                f: |rt, line| rt.describe_service(line),
            },
        );

        cmds.insert(
            "call",
            Cmd {
                help: "Call a service method",
                usage: "call <service_id> <method> [payload]",
                f: |rt, line| rt.call_service_cmd(line),
            },
        );

        cmds.insert(
            "quit",
            Cmd {
                help: "Exit engine",
                usage: "quit",
                f: |rt, _| {
                    rt.exit_requested.store(true, Ordering::Release);
                    Ok("exit requested".into())
                },
            },
        );

        Self {
            cmds,
            dyn_cmds: std::sync::Mutex::new(BTreeMap::new()),
            method_cache: std::sync::Mutex::new(BTreeMap::new()),
            exit_requested: AtomicBool::new(false),
        }
    }

    fn help_text(&self) -> Result<String, String> {
        let mut out = String::new();
        out.push_str("Built-in:\n");
        for (name, c) in &self.cmds {
            out.push_str("  ");
            out.push_str(name);
            out.push_str("  - ");
            out.push_str(c.help);
            out.push('\n');
        }

        let dyn_cmds = self.dyn_cmds.lock().unwrap();
        if !dyn_cmds.is_empty() {
            out.push('\n');
            out.push_str("From services:\n");
            for (name, c) in dyn_cmds.iter() {
                out.push_str("  ");
                out.push_str(name);
                out.push_str("  - ");
                out.push_str(&c.help);
                out.push('\n');
            }
        }

        Ok(out.trim_end().to_string())
    }

    fn exec(&self, line: &str) -> Result<String, String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(String::new());
        }

        let mut it = line.split_whitespace();
        let head = it.next().unwrap_or("");

        if let Some(d) = self.dyn_cmds.lock().unwrap().get(head).cloned() {
            let args = it.collect::<Vec<_>>().join(" ");
            let payload = match d.payload {
                DynPayload::Empty => Vec::new(),
                DynPayload::Raw => args.into_bytes(),
            };
            return self.call_service_raw(&d.service_id, &d.method, &payload);
        }

        if let Some(c) = self.cmds.get(head) {
            return (c.f)(self, line);
        }

        Err(format!("unknown command: {head}"))
    }

    fn suggest(&self, input: &str) -> SuggestResponse {
        let raw = input;
        let s = raw.trim_start();
        let ends_with_space = raw.ends_with(' ');

        let mut items = Vec::new();

        let tokens: Vec<&str> = s.split_whitespace().collect();
        if tokens.is_empty() {
            self.suggest_first_token("", &mut items);
            items.sort_by(|a, b| a.display.cmp(&b.display));
            return SuggestResponse {
                signature: String::new(),
                items,
            };
        }

        let head = tokens[0];
        if tokens.len() == 1 && !ends_with_space {
            self.suggest_first_token(head, &mut items);
            items.sort_by(|a, b| a.display.cmp(&b.display));
            return SuggestResponse {
                signature: String::new(),
                items,
            };
        }

        if head == "describe" {
            let prefix = if tokens.len() >= 2 { tokens[1] } else { "" };
            let signature = self
                .cmds
                .get("describe")
                .map(|c| c.usage.to_string())
                .unwrap_or_default();

            for sid in self.complete_service_id(prefix) {
                let insert = format!("describe {} ", sid);
                items.push(SuggestItem {
                    kind: "service".into(),
                    display: sid.clone(),
                    insert,
                    help: "service id".into(),
                    usage: "describe <service_id>".into(),
                });
            }

            return SuggestResponse { signature, items };
        }

        if head == "call" {
            let signature = self
                .cmds
                .get("call")
                .map(|c| c.usage.to_string())
                .unwrap_or_default();

            let sid = if tokens.len() >= 2 { tokens[1] } else { "" };
            let want_methods = tokens.len() >= 3 || ends_with_space && tokens.len() == 2;

            if sid.is_empty() || !want_methods {
                let prefix = sid;
                for s in self.complete_service_id(prefix) {
                    items.push(SuggestItem {
                        kind: "service".into(),
                        display: s.clone(),
                        insert: format!("call {} ", s),
                        help: "service id".into(),
                        usage: "call <service_id> <method> [payload]".into(),
                    });
                }
                return SuggestResponse { signature, items };
            }

            let method_prefix = if tokens.len() >= 3 { tokens[2] } else { "" };
            for m in self.complete_method(sid, method_prefix) {
                items.push(SuggestItem {
                    kind: "method".into(),
                    display: m.clone(),
                    insert: format!("call {} {} ", sid, m),
                    help: "service method".into(),
                    usage: "call <service_id> <method> [payload]".into(),
                });
            }

            return SuggestResponse { signature, items };
        }

        if let Some(c) = self.cmds.get(head) {
            let signature = c.usage.to_string();
            return SuggestResponse { signature, items };
        }

        if let Some(d) = self.dyn_cmds.lock().unwrap().get(head) {
            return SuggestResponse {
                signature: d.usage.clone(),
                items,
            };
        }

        SuggestResponse {
            signature: String::new(),
            items,
        }
    }

    fn suggest_first_token(&self, prefix: &str, out: &mut Vec<SuggestItem>) {
        for (name, c) in &self.cmds {
            if name.starts_with(prefix) {
                let insert = if c.usage.contains('<') {
                    format!("{} ", name)
                } else {
                    name.to_string()
                };
                out.push(SuggestItem {
                    kind: "command".into(),
                    display: (*name).to_string(),
                    insert,
                    help: c.help.to_string(),
                    usage: c.usage.to_string(),
                });
            }
        }

        for (name, c) in self.dyn_cmds.lock().unwrap().iter() {
            if name.starts_with(prefix) {
                let insert = if c.usage.contains('<') {
                    format!("{} ", name)
                } else {
                    name.to_string()
                };
                out.push(SuggestItem {
                    kind: "command".into(),
                    display: name.clone(),
                    insert,
                    help: c.help.clone(),
                    usage: c.usage.clone(),
                });
            }
        }
    }

    fn complete(&self, input: &str) -> Vec<String> {
        let s = input.trim_start();

        if let Some(rest) = s.strip_prefix("describe ") {
            return self.complete_service_id(rest.trim());
        }

        if let Some(rest) = s.strip_prefix("call ") {
            let mut parts = rest.split_whitespace();
            let sid = parts.next().unwrap_or("");
            let after_sid = rest[sid.len()..].trim_start();

            if sid.is_empty() || after_sid.is_empty() {
                return self.complete_service_id(sid);
            }

            let method_prefix = after_sid.split_whitespace().next().unwrap_or("");
            return self.complete_method(sid, method_prefix);
        }

        let head = s.split_whitespace().next().unwrap_or("");
        let mut out = Vec::new();

        for k in self.cmds.keys() {
            if k.starts_with(head) {
                out.push(k.to_string());
            }
        }
        for k in self.dyn_cmds.lock().unwrap().keys() {
            if k.starts_with(head) {
                out.push(k.to_string());
            }
        }

        out.sort();
        out.dedup();
        out
    }

    fn complete_service_id(&self, prefix: &str) -> Vec<String> {
        let c = host_context::ctx();
        let g = c.services.lock().unwrap();
        let mut v: Vec<String> = g
            .keys()
            .filter(|id| id.starts_with(prefix))
            .cloned()
            .collect();
        v.sort();
        v
    }

    fn complete_method(&self, service_id: &str, prefix: &str) -> Vec<String> {
        self.ensure_method_cache(service_id);
        let g = self.method_cache.lock().unwrap();
        let Some(methods) = g.get(service_id) else {
            return Vec::new();
        };
        let mut out: Vec<String> = methods
            .iter()
            .filter(|m| m.starts_with(prefix))
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn ensure_method_cache(&self, service_id: &str) {
        if self.method_cache.lock().unwrap().contains_key(service_id) {
            return;
        }

        let json = match self.describe_raw(service_id) {
            Ok(v) => v,
            Err(_) => {
                let _ = self
                    .method_cache
                    .lock()
                    .unwrap()
                    .insert(service_id.to_string(), Vec::new());
                return;
            }
        };

        let mut methods = Vec::new();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(arr) = val.get("methods").and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        methods.push(name.to_string());
                    }
                }
            }
        }

        methods.sort();
        methods.dedup();
        let _ = self
            .method_cache
            .lock()
            .unwrap()
            .insert(service_id.to_string(), methods);
    }

    fn refresh_dyn_commands(&self) {
        let mut out: BTreeMap<String, DynCommand> = BTreeMap::new();
        let mut methods: BTreeMap<String, Vec<String>> = BTreeMap::new();

        let c = host_context::ctx();
        let services = c.services.lock().unwrap();
        for (id, svc) in services.iter() {
            let describe = svc.describe().to_string();

            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&describe) {
                if let Some(arr) = v.get("methods").and_then(|x| x.as_array()) {
                    let mut mm = Vec::new();
                    for m in arr {
                        if let Some(name) = m.get("name").and_then(|x| x.as_str()) {
                            mm.push(name.to_string());
                        }
                    }
                    mm.sort();
                    mm.dedup();
                    methods.insert(id.clone(), mm);
                }

                let console = v.get("console");
                let commands = console.and_then(|c| c.get("commands")).and_then(|c| c.as_array());
                if let Some(cmds) = commands {
                    for c in cmds {
                        let Ok(entry) = serde_json::from_value::<ConsoleCmdEntry>(c.clone()) else {
                            continue;
                        };

                        let kind = entry.kind.as_deref().unwrap_or("service_call");
                        if kind != "service_call" {
                            continue;
                        }

                        let sid = entry.service_id.clone().unwrap_or_else(|| id.clone());
                        let method = entry.method.clone().unwrap_or_default();
                        if method.is_empty() {
                            continue;
                        }

                        let payload = match entry.payload.as_deref() {
                            Some("empty") => DynPayload::Empty,
                            _ => DynPayload::Raw,
                        };

                        let usage = entry
                            .usage
                            .clone()
                            .unwrap_or_else(|| format!("{} <args>", entry.name));
                        let help = entry
                            .help
                            .clone()
                            .unwrap_or_else(|| format!("{sid}::{method}"));

                        out.insert(
                            entry.name,
                            DynCommand {
                                help,
                                usage,
                                service_id: sid,
                                method,
                                payload,
                            },
                        );
                    }
                }
            }
        }

        *self.dyn_cmds.lock().unwrap() = out;
        *self.method_cache.lock().unwrap() = methods;
    }

    fn describe_raw(&self, service_id: &str) -> Result<String, String> {
        let c = host_context::ctx();
        let g = c.services.lock().unwrap();
        let svc = g
            .get(service_id)
            .ok_or_else(|| format!("unknown service: {service_id}"))?;
        Ok(svc.describe().to_string())
    }

    fn describe_service(&self, line: &str) -> Result<String, String> {
        let mut it = line.split_whitespace();
        let _ = it.next();
        let sid = it.next().unwrap_or("").trim();
        if sid.is_empty() {
            return Err("usage: describe <service_id>".into());
        }

        let raw = self.describe_raw(sid)?;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            return Ok(serde_json::to_string_pretty(&v).unwrap_or(raw));
        }
        Ok(raw)
    }

    fn call_service_cmd(&self, line: &str) -> Result<String, String> {
        let mut it = line.split_whitespace();
        let _ = it.next();
        let sid = it.next().unwrap_or("").trim();
        let method = it.next().unwrap_or("").trim();
        let payload = it.collect::<Vec<_>>().join(" ");
        if sid.is_empty() || method.is_empty() {
            return Err("usage: call <service_id> <method> [payload]".into());
        }
        self.call_service_raw(sid, method, payload.as_bytes())
    }

    fn call_service_raw(&self, service_id: &str, method: &str, payload: &[u8]) -> Result<String, String> {
        let c = host_context::ctx();
        let g = c.services.lock().unwrap();
        let svc = g
            .get(service_id)
            .ok_or_else(|| format!("unknown service: {service_id}"))?;

        let res = svc.call(RString::from(method), Blob::from(payload.to_vec()));
        match res.into_result() {
            Ok(b) => {
                let bytes = b.into_vec();
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    return Ok(serde_json::to_string_pretty(&v)
                        .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string()));
                }
                Ok(String::from_utf8_lossy(&bytes).to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn take_exit_requested(&self) -> bool {
        self.exit_requested.swap(false, Ordering::AcqRel)
    }
}

struct CommandService {
    rt: Arc<ConsoleRuntime>,
}

impl ServiceV1 for CommandService {
    fn id(&self) -> CapabilityId {
        RString::from(COMMAND_SERVICE_ID)
    }

    fn describe(&self) -> RString {
        RString::from(
            json!({
                "id": COMMAND_SERVICE_ID,
                "version": 2,
                "methods": [
                    { "name": method::EXEC, "payload": "utf8 line", "returns": "json {ok, output?, error?}" },
                    { "name": method::COMPLETE, "payload": "utf8 prefix", "returns": "json {items:[string]}" },
                    { "name": method::SUGGEST, "payload": "utf8 input", "returns": "json SuggestResponse" },
                    { "name": method::REFRESH, "payload": "empty", "returns": "json {ok:true}" }
                ],
                "console": {
                    "commands": [
                        { "name": "help", "help": "List commands", "usage": "help" },
                        { "name": "services", "help": "List services", "usage": "services" },
                        { "name": "refresh", "help": "Refresh console commands", "usage": "refresh" },
                        { "name": "describe", "help": "Describe a service", "usage": "describe <service_id>" },
                        { "name": "call", "help": "Call a service method", "usage": "call <service_id> <method> [payload]" },
                        { "name": "quit", "help": "Exit engine", "usage": "quit" }
                    ]
                }
            })
                .to_string(),
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.to_string().as_str() {
            method::EXEC => {
                let line = String::from_utf8_lossy(payload.as_slice());
                let out = self.rt.exec(&line);
                let resp = match out {
                    Ok(v) => json!({ "ok": true, "output": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                };
                RResult::ROk(Blob::from(resp.to_string().into_bytes()))
            }
            method::COMPLETE => {
                let p = String::from_utf8_lossy(payload.as_slice());
                let v = self.rt.complete(&p);
                RResult::ROk(Blob::from(json!({ "items": v }).to_string().into_bytes()))
            }
            method::SUGGEST => {
                let p = String::from_utf8_lossy(payload.as_slice());
                let r = self.rt.suggest(&p);
                let bytes = serde_json::to_vec(&r).unwrap_or_default();
                RResult::ROk(Blob::from(bytes))
            }
            method::REFRESH => {
                self.rt.refresh_dyn_commands();
                RResult::ROk(Blob::from(json!({ "ok": true }).to_string().into_bytes()))
            }
            _ => RResult::RErr(RString::from("unknown method")),
        }
    }
}

static RT: OnceLock<Arc<ConsoleRuntime>> = OnceLock::new();

pub fn init_console_service() {
    let rt = RT.get_or_init(|| Arc::new(ConsoleRuntime::new())).clone();
    rt.refresh_dyn_commands();
    let svc = CommandService { rt };
    let dyn_svc = ServiceV1Dyn::from_value(svc, abi_stable::sabi_trait::TD_Opaque);
    let _ = host_api::host_register_service_impl(dyn_svc, false);
}

pub fn take_exit_requested() -> bool {
    RT.get().map(|r| r.take_exit_requested()).unwrap_or(false)
}