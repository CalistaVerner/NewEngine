use crate::error::{EngineError, EngineResult};

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Type-safe storage for module APIs and shared engine handles.
///
/// Design note:
/// - Values are stored as `Arc<dyn Any + Send + Sync>` keyed by `TypeId`.
/// - For trait objects (e.g. `Arc<dyn CefApi>`), store `Arc<CefApiRef>`
///   because `Resources::insert<T>` expects `Arc<T>`.
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Resources {
    #[inline]
    pub fn insert<T: Any + Send + Sync>(&mut self, value: Arc<T>) {
        self.map.insert(TypeId::of::<T>(), value);
    }

    #[inline]
    pub fn insert_once<T: Any + Send + Sync>(&mut self, value: Arc<T>) -> EngineResult<()> {
        let k = TypeId::of::<T>();
        if self.map.contains_key(&k) {
            return Err(EngineError::Other("resource already exists".to_string()));
        }
        self.map.insert(k, value);
        Ok(())
    }

    #[inline]
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.map.get(&TypeId::of::<T>()).and_then(|v| {
            let v = v.clone();
            v.downcast::<T>().ok()
        })
    }

    #[inline]
    pub fn get_required<T: Any + Send + Sync>(&self, name: &'static str) -> EngineResult<Arc<T>> {
        self.get::<T>()
            .ok_or_else(|| EngineError::Other(format!("required resource missing: {name}")))
    }

    #[inline]
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    #[inline]
    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<Arc<T>> {
        self.map.remove(&TypeId::of::<T>()).and_then(|v| v.downcast::<T>().ok())
    }

    #[inline]
    pub fn take_required<T: Any + Send + Sync>(&mut self, name: &'static str) -> EngineResult<Arc<T>> {
        self.remove::<T>()
            .ok_or_else(|| EngineError::Other(format!("required resource missing: {name}")))
    }
}