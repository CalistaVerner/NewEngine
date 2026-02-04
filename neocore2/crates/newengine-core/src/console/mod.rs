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
}

type CmdFn = fn(&ConsoleRuntime, &str) -> Result<String, String>;

struct Cmd {
    help: &'static str,
    f: CmdFn,
}

pub struct ConsoleRuntime {
    cmds: BTreeMap<&'static str, Cmd>,
    exit_requested: AtomicBool,
}

impl ConsoleRuntime {
    fn new() -> Self {
        let mut cmds = BTreeMap::new();

        cmds.insert(
            "help",
            Cmd {
                help: "List commands",
                f: |_, _| Ok("help\nservices\nquit".into()),
            },
        );

        cmds.insert(
            "services",
            Cmd {
                help: "List services",
                f: |_, _| {
                    let c = host_context::ctx();
                    let g = c.services.lock().unwrap();
                    Ok(g.keys().cloned().collect::<Vec<_>>().join("\n"))
                },
            },
        );

        cmds.insert(
            "quit",
            Cmd {
                help: "Exit engine",
                f: |rt, _| {
                    rt.exit_requested.store(true, Ordering::Release);
                    Ok("exit requested".into())
                },
            },
        );

        Self {
            cmds,
            exit_requested: AtomicBool::new(false),
        }
    }

    fn exec(&self, line: &str) -> Result<String, String> {
        let cmd = line.trim();
        let c = self
            .cmds
            .get(cmd)
            .ok_or_else(|| format!("unknown command: {cmd}"))?;
        (c.f)(self, line)
    }

    fn complete(&self, prefix: &str) -> Vec<String> {
        self.cmds
            .keys()
            .filter(|k| k.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
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
                "console": {
                    "commands": [
                        { "name": "help" },
                        { "name": "services" },
                        { "name": "quit" }
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
            _ => RResult::RErr(RString::from("unknown method")),
        }
    }
}

static RT: OnceLock<Arc<ConsoleRuntime>> = OnceLock::new();

pub fn init_console_service() {
    let rt = RT.get_or_init(|| Arc::new(ConsoleRuntime::new())).clone();
    let svc = CommandService { rt };
    let dyn_svc = ServiceV1Dyn::from_value(svc, abi_stable::sabi_trait::TD_Opaque);
    let _ = host_api::host_register_service_impl(dyn_svc, false);
}

pub fn take_exit_requested() -> bool {
    RT.get().map(|r| r.take_exit_requested()).unwrap_or(false)
}
