use crossbeam_channel::unbounded;

use newengine_core::{
    AssetManagerConfig, Bus, ConfigPaths, Engine, EngineConfig, EngineResult, Services,
    ShutdownToken, StartupConfig, StartupDefaults, StartupLoader, WindowPlacement,
};
use newengine_modules_logging::{ConsoleLoggerConfig, ConsoleLoggerModule};
use newengine_modules_render_vulkan_ash::{VulkanAshRenderModule, VulkanRenderConfig};
use newengine_platform_winit::{run_winit_app_with_config, WinitAppConfig, WinitWindowPlacement};

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

fn build_engine_from_startup(startup: &StartupConfig) -> EngineResult<Engine<()>> {
    let (tx, rx) = unbounded::<()>();
    let bus: Bus<()> = Bus::new(tx, rx);

    let services: Box<dyn Services> = Box::new(AppServices::new());
    let shutdown = ShutdownToken::new();

    let assets_root = startup.assets_root.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("assets")
    });

    let mut assets = AssetManagerConfig::new(assets_root);
    if let Some(steps) = startup.asset_pump_steps {
        assets = assets.with_pump_steps(steps);
    }
    if let Some(enabled) = startup.asset_filesystem_source {
        assets = assets.with_filesystem_source(enabled);
    }

    let config = EngineConfig::new(16, assets).with_plugins_dir(startup.modules_dir.clone());

    let mut engine: Engine<()> = Engine::new_with_config(config, services, bus, shutdown)?;

    engine.register_module(Box::new(ConsoleLoggerModule::new(
        ConsoleLoggerConfig::from_env(),
    )))?;

    Ok(engine)
}

fn main() -> EngineResult<()> {
    // App provides only path + defaults
    let paths = ConfigPaths::from_startup_str("config.json");
    let defaults = StartupDefaults {
        log_level: Some("info".to_owned()),
        window_title: Some("NewEngine Editor".to_owned()),
        window_size: Some((1600, 900)),
        window_placement: None,
        modules_dir: None,
        assets_root: None,
        asset_pump_steps: Some(8),
        asset_filesystem_source: Some(true),
        render_backend: Some("vulkan_ash".to_owned()),
        render_clear_color: Some([0.0, 0.0, 0.0, 1.0]),
        render_debug_text: None,
    };

    // Single reusable call: returns config + ready-to-log report
    let (startup, report) = StartupLoader::load_json(&paths, &defaults)?;

    // IMPORTANT: engine logger isn't installed yet (ConsoleLoggerModule starts after engine.start()).
    // So we must print startup report directly to stdout/stderr to guarantee visibility.
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

    // Use resolved config (apply result)
    let title = startup
        .window_title
        .clone()
        .unwrap_or_else(|| "NewEngine".to_owned());
    let (w, h) = startup.window_size.unwrap_or((1280, 720));
    let placement = match startup.window_placement.clone().unwrap_or(WindowPlacement::Default) {
        WindowPlacement::Centered { offset } => WinitWindowPlacement::Centered { offset },
        WindowPlacement::Default => WinitWindowPlacement::OsDefault,
    };

    let cfg = WinitAppConfig {
        title,
        size: (w, h),
        placement,
        ..WinitAppConfig::default()
    };

    let engine = build_engine_from_startup(&startup)?;

    run_winit_app_with_config(engine, cfg, |engine| {
        let backend = startup
            .render_backend
            .clone()
            .unwrap_or_else(|| "vulkan_ash".to_owned());
        if backend.eq_ignore_ascii_case("vulkan_ash") {
            let config = VulkanRenderConfig {
                clear_color: startup
                    .render_clear_color
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]),
                debug_text: startup.render_debug_text.clone(),
            };
            engine.register_module(Box::new(VulkanAshRenderModule::new(config)))?;
        } else {
            return Err(EngineError::other(format!(
                "unsupported render backend '{backend}'"
            )));
        }
        Ok(())
    })?;

    println!("engine stopped");
    Ok(())
}
