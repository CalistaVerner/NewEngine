use crossbeam_channel::{Receiver, Sender};

pub struct Bus<E: Send + 'static> {
    tx: Sender<E>,
    rx: Receiver<E>,
}

impl<E: Send + 'static> Bus<E> {
    #[inline]
    pub fn new(tx: Sender<E>, rx: Receiver<E>) -> Self {
        Self { tx, rx }
    }

    #[inline]
    pub fn send(&self, ev: E) {
        let _ = self.tx.send(ev);
    }

    #[inline]
    pub fn try_recv(&self) -> Option<E> {
        self.rx.try_recv().ok()
    }

    #[inline]
    pub fn drain_into(&self, out: &mut Vec<E>) {
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
    }
}