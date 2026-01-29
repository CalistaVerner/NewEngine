use crate::error::{EngineError, EngineResult};

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type-safe storage for module APIs and shared engine handles.
///
/// Stores owned values in a `Box<dyn Any + Send + Sync>` keyed by `TypeId`.
/// For shared APIs, store the final handle type (e.g. `Arc<dyn CefApi>`) as the value.
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    #[inline]
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync + 'static,
    {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    #[inline]
    pub fn insert_once<T>(&mut self, value: T) -> EngineResult<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let k = TypeId::of::<T>();
        if self.map.contains_key(&k) {
            return Err(EngineError::Other("resource already exists".to_string()));
        }
        self.map.insert(k, Box::new(value));
        Ok(())
    }

    #[inline]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.map.get(&TypeId::of::<T>()).and_then(|v| v.downcast_ref::<T>())
    }

    #[inline]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.map.get_mut(&TypeId::of::<T>()).and_then(|v| v.downcast_mut::<T>())
    }

    #[inline]
    pub fn get_required<T>(&self, name: &'static str) -> EngineResult<&T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.get::<T>()
            .ok_or_else(|| EngineError::Other(format!("required resource missing: {name}")))
    }

    #[inline]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.map.contains_key(&TypeId::of::<T>())
    }

    #[inline]
    pub fn remove<T>(&mut self) -> Option<T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok())
            .map(|b| *b)
    }

    #[inline]
    pub fn take_required<T>(&mut self, name: &'static str) -> EngineResult<T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.remove::<T>()
            .ok_or_else(|| EngineError::Other(format!("required resource missing: {name}")))
    }
}
