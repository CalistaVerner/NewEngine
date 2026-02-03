use crossbeam_channel::unbounded;

use newengine_core::{
    AssetManagerConfig, Bus, ConfigPaths, Engine, EngineConfig, EngineError, EngineResult,
    Services, ShutdownToken, StartupConfig, StartupLoader,
};

use newengine_modules_logging::{ConsoleLoggerConfig, ConsoleLoggerModule};
use newengine_modules_render_vulkan_ash::{VulkanAshRenderModule, VulkanRenderConfig};
use newengine_platform_winit::{
    run_winit_app_with_config, UiBuildFn, WinitAppConfig, WinitWindowPlacement,
};

mod ui;

const FIXED_DT_MS: u32 = 16;

struct AppServices;

impl AppServices {
    #[inline]
    fn new() -> Self {
        Self
    }
}

impl Services for AppServices {
    #[inline]
    fn logger(&self) -> &dyn log::Log {
        log::logger()
    }
}

#[inline]
fn winit_config_from_startup(startup: &StartupConfig) -> WinitAppConfig {
    let placement = match startup.window_placement {
        newengine_core::startup::WindowPlacement::Default => WinitWindowPlacement::OsDefault,
        newengine_core::startup::WindowPlacement::Centered { offset } => {
            WinitWindowPlacement::Centered { offset }
        }
    };

    WinitAppConfig {
        title: startup.window_title.clone(),
        size: startup.window_size,
        placement,
        ui_backend: startup.ui_backend.clone(),
    }
}

#[inline]
fn ui_build_from_startup(startup: &StartupConfig) -> Option<Box<dyn UiBuildFn>> {
    match startup.ui_backend {
        newengine_core::startup::UiBackend::Disabled => None,
        _ => Some(Box::new(ui::EditorUiBuild::default())),
    }
}

#[inline]
fn register_render_from_startup(
    engine: &mut Engine<()>,
    startup: &StartupConfig,
) -> EngineResult<()> {
    let backend = startup.render_backend.trim();
    if backend.eq_ignore_ascii_case("vulkan_ash") || backend.eq_ignore_ascii_case("vulkan") {
        let debug_text = startup.render_debug_text.trim();
        let config = VulkanRenderConfig {
            clear_color: startup.render_clear_color,
            debug_text: (!debug_text.is_empty()).then(|| debug_text.to_owned()),
        };

        engine.register_module(Box::new(VulkanAshRenderModule::new(config)))?;
        return Ok(());
    }

    Err(EngineError::other(format!(
        "unsupported render backend '{backend}'"
    )))
}

fn build_engine_from_startup(startup: &StartupConfig) -> EngineResult<Engine<()>> {
    let (tx, rx) = unbounded::<()>();
    let bus: Bus<()> = Bus::new(tx, rx);

    let services: Box<dyn Services> = Box::new(AppServices::new());
    let shutdown = ShutdownToken::new();

    let assets = AssetManagerConfig::new(startup.assets_root.clone())
        .with_pump_steps(startup.asset_pump_steps)
        .with_filesystem_source(startup.asset_filesystem_source);

    let config =
        EngineConfig::new(FIXED_DT_MS, assets).with_plugins_dir(Some(startup.modules_dir.clone()));

    let mut engine: Engine<()> = Engine::new_with_config(config, services, bus, shutdown)?;

    engine.register_module(Box::new(ConsoleLoggerModule::new(
        ConsoleLoggerConfig::from_env(),
    )))?;

    Ok(engine)
}

fn main() -> EngineResult<()> {
    let paths = ConfigPaths::from_startup_str("config.json");
    let (startup, report) = StartupLoader::load_json(&paths)?;

    println!(
        "startup: loaded source={:?} file={:?} resolved_from={:?} overrides={}",
        report.source,
        report.file,
        report.resolved_from,
        report.overrides.len()
    );
    for ov in report.overrides.iter() {
        println!("startup: override {}: '{}' -> '{}'", ov.key, ov.from, ov.to);
    }

    let engine = build_engine_from_startup(&startup)?;
    let winit_cfg = winit_config_from_startup(&startup);
    let ui_build = ui_build_from_startup(&startup);

    run_winit_app_with_config(engine, winit_cfg, ui_build, move |engine| {
        register_render_from_startup(engine, &startup)
    })?;

    println!("engine stopped");
    Ok(())
}
