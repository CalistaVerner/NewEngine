use log::info;
use newengine_assets::{AssetStore, FileSystemSource, PumpBudget};
use std::path::PathBuf;
use std::sync::Arc;

pub struct AssetManager {
    store: Arc<AssetStore>,
    budget: PumpBudget,
}

impl AssetManager {
    #[inline]
    pub fn new_default(root: PathBuf) -> Self {
        info!(target: "assets", "manager.init root='{}'", root.display());

        let store = Arc::new(AssetStore::new());

        info!(
            target: "assets",
            "manager.source.register kind='filesystem' root='{}'",
            root.display()
        );
        store.add_source(Arc::new(FileSystemSource::new(root)));

        let budget = PumpBudget::steps(8);
        info!(target: "assets", "manager.budget steps={}", budget.steps);

        Self { store, budget }
    }

    #[inline]
    pub fn store(&self) -> &Arc<AssetStore> {
        &self.store
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
}