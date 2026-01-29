// FILE: src/module.rs
use crate::commands::{EngineEvent, Request, Response};
use crate::services::Services;

use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};

/// Type-erased resource container (service locator style).
/// Stored values must be `Send` to allow moving the engine across threads if needed.
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl Resources {
    #[inline]
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    #[inline]
    pub fn insert<T: Any + Send>(&mut self, v: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(v));
    }

    #[inline]
    pub fn contains<T: Any + Send>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    #[inline]
    pub fn remove<T: Any + Send>(&mut self) -> Option<T> {
        let boxed = self.map.remove(&TypeId::of::<T>())?;
        boxed.downcast::<T>().ok().map(|b| *b)
    }

    #[inline]
    pub fn get<T: Any + Send>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    #[inline]
    pub fn get_mut<T: Any + Send>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }
}

/// Runtime context passed to modules.
///
/// Two lifetimes:
/// - `'s` for immutable services
/// - `'r` for runtime mutable access (bus/resources)
pub struct ModuleCtx<'s, 'r> {
    pub services: &'s dyn Services,
    pub bus: &'r mut dyn Bus,
    pub resources: &'r mut Resources,
}

/// Minimal runtime bus surface that request handlers are allowed to use.
/// This prevents borrowing `&mut BusImpl` twice inside `request()`.
pub trait BusRuntime {
    fn emit(&mut self, ev: EngineEvent);
    fn poll_event(&mut self) -> Option<EngineEvent>;
}

/// Request handler signature.
/// Handlers must be object-safe and must not depend on `ModuleCtx` lifetimes.
pub type RequestHandler = Box<
    dyn FnMut(&dyn Services, &mut dyn BusRuntime, &mut Resources, &Request) -> Option<Response>
    + Send,
>;

/// Engine event bus.
/// `request()` is object-safe and stable for any module type.
pub trait Bus {
    fn emit(&mut self, ev: EngineEvent);
    fn poll_event(&mut self) -> Option<EngineEvent>;

    fn register_handler(&mut self, owner: &'static str, f: RequestHandler);

    fn request(&mut self, services: &dyn Services, resources: &mut Resources, req: Request)
               -> Response;
}

/// Default engine bus implementation.
pub struct BusImpl {
    events: VecDeque<EngineEvent>,
    handlers: Vec<(&'static str, RequestHandler)>,
}

impl BusImpl {
    #[inline]
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            handlers: Vec::new(),
        }
    }

    #[inline]
    pub fn len_events(&self) -> usize {
        self.events.len()
    }

    #[inline]
    pub fn len_handlers(&self) -> usize {
        self.handlers.len()
    }
}

impl BusRuntime for BusImpl {
    #[inline]
    fn emit(&mut self, ev: EngineEvent) {
        self.events.push_back(ev);
    }

    #[inline]
    fn poll_event(&mut self) -> Option<EngineEvent> {
        self.events.pop_front()
    }
}

impl Bus for BusImpl {
    #[inline]
    fn emit(&mut self, ev: EngineEvent) {
        BusRuntime::emit(self, ev);
    }

    #[inline]
    fn poll_event(&mut self) -> Option<EngineEvent> {
        BusRuntime::poll_event(self)
    }

    #[inline]
    fn register_handler(&mut self, owner: &'static str, f: RequestHandler) {
        self.handlers.push((owner, f));
    }

    fn request(
        &mut self,
        services: &dyn Services,
        resources: &mut Resources,
        req: Request,
    ) -> Response {
        struct RuntimeView<'a> {
            events: &'a mut VecDeque<EngineEvent>,
        }

        impl BusRuntime for RuntimeView<'_> {
            #[inline]
            fn emit(&mut self, ev: EngineEvent) {
                self.events.push_back(ev);
            }

            #[inline]
            fn poll_event(&mut self) -> Option<EngineEvent> {
                self.events.pop_front()
            }
        }

        let mut rt = RuntimeView { events: &mut self.events };

        for (_owner, h) in self.handlers.iter_mut() {
            if let Some(resp) = (h)(services, &mut rt as &mut dyn BusRuntime, resources, &req) {
                return resp;
            }
        }

        Response::Err(format!("no handler for request: {:?}", req))
    }
}

/// Module interface.
/// Modules can be anything: rendering, audio, scripting, networking, tooling, etc.
pub trait Module: Send {
    fn id(&self) -> &'static str;

    fn register(&mut self, _ctx: &mut ModuleCtx<'_, '_>) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_event(&mut self, _ctx: &mut ModuleCtx<'_, '_>, _ev: &EngineEvent) -> anyhow::Result<()> {
        Ok(())
    }

    fn update(&mut self, _ctx: &mut ModuleCtx<'_, '_>, _dt: f32) -> anyhow::Result<()> {
        Ok(())
    }

    fn fixed_update(&mut self, _ctx: &mut ModuleCtx<'_, '_>, _dt: f32) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(&mut self, _ctx: &mut ModuleCtx<'_, '_>) -> anyhow::Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut ModuleCtx<'_, '_>) -> anyhow::Result<()> {
        Ok(())
    }
}