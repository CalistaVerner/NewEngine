use crate::sched::Scheduler;

use super::{Bus, Resources, Services};

/// Context passed to modules.
///
/// This prevents modules from taking `&mut Engine` (god object problem).
pub struct ModuleCtx<'a, E: Send + 'static> {
    services: &'a dyn Services,
    resources: &'a mut Resources,
    bus: &'a Bus<E>,
    scheduler: &'a mut Scheduler,
    exit: &'a mut bool,
}

impl<'a, E: Send + 'static> ModuleCtx<'a, E> {
    #[inline]
    pub(crate) fn new(
        services: &'a dyn Services,
        resources: &'a mut Resources,
        bus: &'a Bus<E>,
        scheduler: &'a mut Scheduler,
        exit: &'a mut bool,
    ) -> Self {
        Self { services, resources, bus, scheduler, exit }
    }

    #[inline]
    pub fn services(&self) -> &dyn Services {
        self.services
    }

    #[inline]
    pub fn resources(&mut self) -> &mut Resources {
        self.resources
    }

    #[inline]
    pub fn bus(&self) -> &Bus<E> {
        self.bus
    }

    #[inline]
    pub fn scheduler(&mut self) -> &mut Scheduler {
        self.scheduler
    }

    #[inline]
    pub fn request_exit(&mut self) {
        *self.exit = true;
    }
}