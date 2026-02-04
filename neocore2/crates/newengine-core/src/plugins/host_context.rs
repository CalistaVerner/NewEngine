#![forbid(unsafe_op_in_unsafe_fn)]

use newengine_assets::AssetStore;
use newengine_plugin_api::ServiceV1Dyn;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub struct HostContext {
    pub(crate) services: Mutex<HashMap<String, Arc<ServiceV1Dyn<'static>>>>,
    pub(crate) asset_store: Arc<AssetStore>,
    services_generation: AtomicU64,
}

static HOST_CTX: OnceLock<Arc<HostContext>> = OnceLock::new();

pub fn init_host_context(asset_store: Arc<AssetStore>) {
    let ctx = Arc::new(HostContext {
        services: Mutex::new(HashMap::new()),
        asset_store,
        services_generation: AtomicU64::new(1),
    });
    let _ = HOST_CTX.set(ctx);
}

#[inline]
pub(crate) fn ctx() -> Arc<HostContext> {
    HOST_CTX
        .get()
        .expect("HostContext not initialized (call init_host_context first)")
        .clone()
}

#[inline]
pub(crate) fn services_generation() -> u64 {
    ctx().services_generation.load(Ordering::Acquire)
}

#[inline]
pub(crate) fn bump_services_generation() {
    ctx().services_generation.fetch_add(1, Ordering::AcqRel);
}