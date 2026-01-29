use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Type-safe storage for module APIs and shared engine handles.
///
/// Notes:
/// - Values are stored as `Arc<dyn Any + Send + Sync>` keyed by `TypeId` of T.
/// - For trait objects (e.g. `Arc<dyn CefApi>`), store them via a dedicated key type:
///   `resources.insert::<CefApiKey>(Arc::new(api_ref));`
///   where `api_ref: Arc<dyn CefApi + Send + Sync>`.
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
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.map.get(&TypeId::of::<T>()).and_then(|v| {
            let v = v.clone();
            v.downcast::<T>().ok()
        })
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
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}