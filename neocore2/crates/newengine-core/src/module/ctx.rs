use crate::events::EventHub;
use crate::frame::Frame;
use crate::module::{Bus, Resources, Services};
use crate::sched::Scheduler;

/// Context passed to modules.
///
/// This prevents modules from taking `&mut Engine`.
pub struct ModuleCtx<'a, E: Send + 'static> {
    services: &'a dyn Services,
    resources: &'a mut Resources,
    bus: &'a Bus<E>,
    events: &'a EventHub,
    scheduler: &'a mut Scheduler,
    exit: &'a mut bool,

    frame: Option<Frame>,
}

impl<'a, E: Send + 'static> ModuleCtx<'a, E> {
    #[inline]
    pub(crate) fn new(
        services: &'a dyn Services,
        resources: &'a mut Resources,
        bus: &'a Bus<E>,
        events: &'a EventHub,
        scheduler: &'a mut Scheduler,
        exit: &'a mut bool,
    ) -> Self {
        Self {
            services,
            resources,
            bus,
            events,
            scheduler,
            exit,
            frame: None,
        }
    }

    #[inline]
    pub fn set_frame(&mut self, frame: &Frame) {
        self.frame = Some(*frame);
    }

    #[inline]
    pub fn frame(&self) -> Option<&Frame> {
        self.frame.as_ref()
    }

    #[inline]
    pub fn services(&self) -> &dyn Services {
        self.services
    }

    #[inline]
    pub fn resources(&mut self) -> &mut Resources {
        self.resources
    }

    /// Commands queue (single-consumer by rules).
    #[inline]
    pub fn bus(&self) -> &Bus<E> {
        self.bus
    }

    /// Multicast events hub (pub/sub).
    #[inline]
    pub fn events(&self) -> &EventHub {
        self.events
    }

    #[inline]
    pub fn scheduler(&mut self) -> &mut Scheduler {
        self.scheduler
    }

    #[inline]
    pub fn request_exit(&mut self) {
        *self.exit = true;
    }

    #[inline]
    pub fn is_exit_requested(&self) -> bool {
        *self.exit
    }
}