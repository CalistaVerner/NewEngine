#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::library::RootModule;
use abi_stable::std_types::{RResult, RString};
use libloading::Library;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use newengine_plugin_api::{
    Blob, CapabilityId, EventSinkV1Dyn, HostApiV1, MethodName, PluginInfo, PluginModuleDyn,
    PluginRootV1Ref, ServiceV1Dyn,
};

/* =============================================================================================
   Plugin manager
   ============================================================================================= */

pub struct LoadedPlugin {
    _path: PathBuf,
    _lib: Library,
    _root: PluginRootV1Ref,
    module: PluginModuleDyn<'static>,
    info: PluginInfo,
}

impl LoadedPlugin {
    #[inline]
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }
}

pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
    started: bool,
    ids: HashSet<String>,
}

impl PluginManager {
    #[inline]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            started: false,
            ids: HashSet::new(),
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &LoadedPlugin> {
        self.plugins.iter()
    }

    pub fn load_default(&mut self, host: HostApiV1) -> Result<(), String> {
        let dir = modules_dir_near_exe()?;
        self.load_dir(&dir, host)
    }

    pub fn load_dir(&mut self, dir: &Path, host: HostApiV1) -> Result<(), String> {
        (host.log_info)(RString::from(format!(
            "plugins: scanning directory '{}'",
            dir.display()
        )));

        std::fs::create_dir_all(dir)
            .map_err(|e| format!("plugins: create_dir_all('{}') failed: {e}", dir.display()))?;

        let mut libs: Vec<PathBuf> = Vec::new();
        let rd = std::fs::read_dir(dir)
            .map_err(|e| format!("plugins: read_dir('{}') failed: {e}", dir.display()))?;

        for ent in rd {
            let ent = ent.map_err(|e| format!("plugins: read_dir entry failed: {e}"))?;
            let p = ent.path();
            if is_dynlib(&p) {
                libs.push(p);
            }
        }

        libs.sort();

        (host.log_info)(RString::from(format!(
            "plugins: found {} candidate(s) in '{}'",
            libs.len(),
            dir.display()
        )));

        let mut loaded = 0usize;
        let mut skipped = 0usize;

        for path in libs {
            (host.log_info)(RString::from(format!(
                "plugins: loading '{}'",
                path.display()
            )));

            let (lib, root) = match load_root_via_libloading(&path) {
                Ok(v) => v,
                Err(e) => {
                    skipped += 1;
                    (host.log_warn)(RString::from(format!(
                        "plugins: SKIP incompatible plugin file='{}': {}",
                        path.display(),
                        e
                    )));
                    continue;
                }
            };

            let mut module = root.create()();
            let info = module.info();

            let id_key = info.id.to_string();
            if self.ids.contains(&id_key) {
                skipped += 1;
                (host.log_warn)(RString::from(format!(
                    "plugins: SKIP duplicate plugin id='{}' file='{}'",
                    info.id,
                    path.display()
                )));
                continue;
            }

            if let Err(e) = module.init(host.clone()).into_result() {
                skipped += 1;
                (host.log_warn)(RString::from(format!(
                    "plugins: SKIP plugin init failed id='{}' ver='{}' file='{}': {}",
                    info.id,
                    info.version,
                    path.display(),
                    e
                )));
                continue;
            }

            self.ids.insert(id_key);

            loaded += 1;
            (host.log_info)(RString::from(format!(
                "plugins: loaded id='{}' ver='{}' from '{}'",
                info.id,
                info.version,
                path.display()
            )));

            self.plugins.push(LoadedPlugin {
                _path: path,
                _lib: lib,
                _root: root,
                module,
                info,
            });
        }

        (host.log_info)(RString::from(format!(
            "plugins: load summary loaded={} skipped={}",
            loaded, skipped
        )));

        Ok(())
    }

    pub fn start_all(&mut self) -> Result<(), String> {
        if self.started {
            return Ok(());
        }

        for p in self.plugins.iter_mut() {
            p.module
                .start()
                .into_result()
                .map_err(|e| format!("plugins: start failed for id='{}': {}", p.info.id, e))?;
        }

        self.started = true;
        Ok(())
    }

    pub fn fixed_update_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module.fixed_update(dt).into_result().map_err(|e| {
                format!("plugins: fixed_update failed for id='{}': {}", p.info.id, e)
            })?;
        }
        Ok(())
    }

    pub fn update_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .update(dt)
                .into_result()
                .map_err(|e| format!("plugins: update failed for id='{}': {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn render_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .render(dt)
                .into_result()
                .map_err(|e| format!("plugins: render failed for id='{}': {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        for p in self.plugins.iter_mut().rev() {
            p.module.shutdown();
        }
        self.plugins.clear();
        self.ids.clear();
        self.started = false;
    }
}

/* =============================================================================================
   Root loading bound to a specific DLL (no global cache)
   ============================================================================================= */

fn load_root_via_libloading(path: &Path) -> Result<(Library, PluginRootV1Ref), String> {
    let lib = unsafe {
        Library::new(path)
            .map_err(|e| format!("load library failed file='{}': {e}", path.display()))?
    };

    let name_primary = <PluginRootV1Ref as RootModule>::NAME;
    let name_fallback = <PluginRootV1Ref as RootModule>::BASE_NAME;

    // Safety: we resolve function pointers from a live library handle and call them immediately.
    unsafe {
        if let Ok(root) = try_get_root_fn(&lib, name_primary) {
            return Ok((lib, root));
        }
        if let Ok(root) = try_get_root_fn(&lib, name_fallback) {
            return Ok((lib, root));
        }
    }

    Err(format!(
        "missing root symbol '{}' (or '{}') in '{}': GetProcAddress failed",
        name_primary,
        name_fallback,
        path.display()
    ))
}

unsafe fn try_get_root_fn(lib: &Library, sym_name: &str) -> Result<PluginRootV1Ref, ()> {
    let mut bytes = Vec::with_capacity(sym_name.len() + 1);
    bytes.extend_from_slice(sym_name.as_bytes());
    bytes.push(0);

    let sym = unsafe {
        lib.get::<unsafe extern "C" fn() -> PluginRootV1Ref>(&bytes)
            .map_err(|_| ())?
    };

    let get_root: unsafe extern "C" fn() -> PluginRootV1Ref = *sym;

    // End the borrow of `lib` before calling/returning anything.
    drop(sym);

    let root = unsafe { get_root() };
    Ok(root)
}

/* =============================================================================================
   Helpers
   ============================================================================================= */

#[inline]
fn is_dynlib(p: &Path) -> bool {
    let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "dll" | "so" | "dylib")
}

#[inline]
pub fn default_host_api() -> HostApiV1 {
    extern "C" fn log_info(msg: RString) {
        log::info!("{}", msg);
    }

    extern "C" fn log_warn(msg: RString) {
        log::warn!("{}", msg);
    }

    extern "C" fn log_error(msg: RString) {
        log::error!("{}", msg);
    }

    extern "C" fn register_service_v1(_svc: ServiceV1Dyn<'static>) -> RResult<(), RString> {
        RResult::ROk(())
    }

    extern "C" fn call_service_v1(
        _id: CapabilityId,
        _method: MethodName,
        _payload: Blob,
    ) -> RResult<Blob, RString> {
        RResult::RErr(RString::from("service not found"))
    }

    extern "C" fn emit_event_v1(_topic: RString, _payload: Blob) -> RResult<(), RString> {
        RResult::ROk(())
    }

    extern "C" fn subscribe_events_v1(_sink: EventSinkV1Dyn<'static>) -> RResult<(), RString> {
        RResult::ROk(())
    }

    HostApiV1 {
        log_info,
        log_warn,
        log_error,
        register_service_v1,
        call_service_v1,
        emit_event_v1,
        subscribe_events_v1,
    }
}

pub fn modules_dir_near_exe() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let base = exe
        .parent()
        .ok_or_else(|| "current_exe has no parent directory".to_string())?;
    Ok(base.to_path_buf())
}