#![forbid(unsafe_op_in_unsafe_fn)]

use std::path::{Path, PathBuf};
use abi_stable::library::RootModule;
use newengine_plugin_api::{HostApiV1, PluginInfo, PluginModule_TO, PluginRootV1_Ref};

pub struct LoadedPlugin {
    _path: PathBuf,
    module: PluginModule_TO<'static, abi_stable::std_types::RBox<()>>,
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
}

impl PluginManager {
    #[inline]
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn load_dir(&mut self, dir: &Path, host: HostApiV1) -> Result<(), String> {
        if !dir.exists() {
            return Ok(());
        }

        let mut libs: Vec<PathBuf> = Vec::new();
        let rd = std::fs::read_dir(dir).map_err(|e| format!("read_dir({}): {e}", dir.display()))?;
        for ent in rd {
            let ent = ent.map_err(|e| e.to_string())?;
            let p = ent.path();
            if is_dynlib(&p) {
                libs.push(p);
            }
        }

        libs.sort();

        for path in libs {
            let root = PluginRootV1_Ref::load_from_file(&path)
                .map_err(|e| format!("load_from_file({}): {e}", path.display()))?;

            let mut module = (root.create())();
            let info = module.info();

            module
                .init(host.clone())
                .into_result()
                .map_err(|e| format!("plugin '{}' init failed: {}", info.id, e))?;

            self.plugins.push(LoadedPlugin {
                _path: path,
                module,
                info,
            });
        }

        Ok(())
    }

    pub fn start_all(&mut self) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .start()
                .into_result()
                .map_err(|e| format!("plugin '{}' start failed: {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn fixed_update_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .fixed_update(dt)
                .into_result()
                .map_err(|e| format!("plugin '{}' fixed_update failed: {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn update_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .update(dt)
                .into_result()
                .map_err(|e| format!("plugin '{}' update failed: {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn render_all(&mut self, dt: f32) -> Result<(), String> {
        for p in self.plugins.iter_mut() {
            p.module
                .render(dt)
                .into_result()
                .map_err(|e| format!("plugin '{}' render failed: {}", p.info.id, e))?;
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        for p in self.plugins.iter_mut().rev() {
            p.module.shutdown();
        }
        self.plugins.clear();
    }
}

#[inline]
fn is_dynlib(p: &Path) -> bool {
    let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "dll" | "so" | "dylib")
}