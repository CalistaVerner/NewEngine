use log::info;
use newengine_assets::{
    AssetBlob, AssetError, AssetEvent, AssetId, AssetKey, AssetSource, AssetState, AssetStore,
    BlobImporterDispatch, FileSystemSource, PumpBudget,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AssetManagerConfig {
    pub root: PathBuf,
    pub pump_steps: u32,
    pub enable_filesystem_source: bool,
}

impl AssetManagerConfig {
    #[inline]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            pump_steps: 8,
            enable_filesystem_source: true,
        }
    }

    #[inline]
    pub fn with_pump_steps(mut self, steps: u32) -> Self {
        self.pump_steps = steps;
        self
    }

    #[inline]
    pub fn with_filesystem_source(mut self, enabled: bool) -> Self {
        self.enable_filesystem_source = enabled;
        self
    }
}

pub struct AssetManager {
    store: Arc<AssetStore>,
    budget: PumpBudget,
}

impl AssetManager {
    #[inline]
    pub fn new_default(root: PathBuf) -> Self {
        Self::new_with_config(AssetManagerConfig::new(root))
    }

    #[inline]
    pub fn new_with_config(config: AssetManagerConfig) -> Self {
        info!(target: "assets", "manager.init root='{}'", config.root.display());

        let store = Arc::new(AssetStore::new());

        if config.enable_filesystem_source {
            info!(
                target: "assets",
                "manager.source.register kind='filesystem' root='{}'",
                config.root.display()
            );
            store.add_source(Arc::new(FileSystemSource::new(config.root)));
        }

        let steps = config.pump_steps.max(1);
        let budget = PumpBudget::steps(steps);
        info!(target: "assets", "manager.budget steps={}", budget.steps);

        Self { store, budget }
    }

    /// Returns a shared handle to the underlying store.
    #[inline]
    pub fn store(&self) -> &Arc<AssetStore> {
        &self.store
    }

    /// Registers an additional asset source.
    #[inline]
    pub fn add_source(&self, source: Arc<dyn AssetSource>) {
        self.store.add_source(source);
    }

    /// Registers a type-erased importer dispatch (usually a plugin-backed service adapter).
    #[inline]
    pub fn add_importer(&self, importer: Arc<dyn BlobImporterDispatch>) {
        self.store.add_importer(importer);
    }

    /// Enqueues an import request.
    #[inline]
    pub fn load(&self, key: AssetKey) -> Result<AssetId, AssetError> {
        self.store.load(key)
    }

    #[inline]
    pub fn state(&self, id: AssetId) -> AssetState {
        self.store.state(id)
    }

    #[inline]
    pub fn get_blob(&self, id: AssetId) -> Option<Arc<AssetBlob>> {
        self.store.get_blob(id)
    }

    #[inline]
    pub fn drain_events(&self) -> Vec<AssetEvent> {
        self.store.drain_events()
    }

    #[inline]
    pub fn set_budget(&mut self, steps: u32) {
        let steps = steps.max(1);
        info!(target: "assets", "manager.budget.update steps={}", steps);
        self.budget = PumpBudget::steps(steps);
    }

    #[inline]
    pub fn pump(&self) {
        self.store.pump(self.budget);
    }

    /// Convenience: pump and return any produced events.
    #[inline]
    pub fn pump_and_drain(&self) -> Vec<AssetEvent> {
        self.pump();
        self.drain_events()
    }
}
