#![forbid(unsafe_op_in_unsafe_fn)]

use newengine_assets::AssetStore;
use newengine_plugin_api::ServiceV1Dyn;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub struct HostContext {
    pub(crate) services: Mutex<HashMap<String, ServiceV1Dyn<'static>>>,
    pub(crate) asset_store: Arc<AssetStore>,
}

static HOST_CTX: OnceLock<Arc<HostContext>> = OnceLock::new();

pub fn init_host_context(asset_store: Arc<AssetStore>) {
    let ctx = Arc::new(HostContext {
        services: Mutex::new(HashMap::new()),
        asset_store,
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
