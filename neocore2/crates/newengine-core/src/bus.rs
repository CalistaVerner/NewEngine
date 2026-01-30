use crossbeam_channel::{Receiver, SendError, Sender};
use std::sync::atomic::{AtomicU64, Ordering};

/// Command queue (single-consumer).
///
/// Producer side: any module can `send/try_send`.
/// Consumer side: by rules, exactly one module (operator) should `try_recv/drain`.
///
/// Runtime-guard:
/// - first consumer becomes the owner,
/// - other consumers will panic in debug builds.
pub struct Bus<E: Send + 'static> {
    tx: Sender<E>,
    rx: Receiver<E>,
    consumer_guard: ConsumerGuard,
}

impl<E: Send + 'static> Bus<E> {
    #[inline]
    pub fn new(tx: Sender<E>, rx: Receiver<E>) -> Self {
        Self {
            tx,
            rx,
            consumer_guard: ConsumerGuard::new(),
        }
    }

    #[inline]
    pub fn try_send(&self, ev: E) -> bool {
        self.tx.try_send(ev).is_ok()
    }

    #[inline]
    pub fn send(&self, ev: E) -> Result<(), SendError<E>> {
        self.tx.send(ev)
    }

    /// Single-consumer receive.
    #[inline]
    pub fn try_recv(&self) -> Option<E> {
        self.consumer_guard.assert_or_claim();
        self.rx.try_recv().ok()
    }

    /// Single-consumer drain.
    #[inline]
    pub fn drain_into(&self, out: &mut Vec<E>) -> usize {
        self.consumer_guard.assert_or_claim();
        let mut n = 0usize;
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
            n += 1;
        }
        n
    }

    /// Single-consumer drain with callback.
    #[inline]
    pub fn drain<F: FnMut(E)>(&self, mut f: F) {
        self.consumer_guard.assert_or_claim();
        while let Ok(ev) = self.rx.try_recv() {
            f(ev);
        }
    }
}

struct ConsumerGuard {
    owner: AtomicU64,
}

impl ConsumerGuard {
    #[inline]
    fn new() -> Self {
        Self {
            owner: AtomicU64::new(0),
        }
    }

    #[inline]
    fn assert_or_claim(&self) {
        let id = consumer_id();
        let cur = self.owner.load(Ordering::Acquire);

        if cur == 0 {
            let _ = self.owner.compare_exchange(0, id, Ordering::AcqRel, Ordering::Acquire);
            return;
        }

        if cur != id {
            // "по правилам": один consumer.
            // В debug это must-fail (иначе будет ад в больших проектах).
            if cfg!(debug_assertions) {
                panic!("Bus<E> single-consumer violation: multiple consumers detected");
            }
        }
    }
}

#[inline]
fn consumer_id() -> u64 {
    // Stable per-thread id. Good enough to detect "another consumer".
    // Not cryptographic, not security-related.
    use std::hash::{Hash, Hasher};
    let tid = std::thread::current().id();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tid.hash(&mut h);
    let v = h.finish();
    if v == 0 { 1 } else { v }
}