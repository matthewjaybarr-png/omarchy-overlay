//! Serialises Windows appearance changes. A theme apply runs on a worker and is dropped when a
//! newer change (another theme, a restore, a settings page) claimed a generation first.
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Mutex, MutexGuard, TryLockError};

static LOCK: Mutex<()> = Mutex::new(());
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Claims the next generation; any change queued under an older one is now stale.
pub fn claim() -> u64 {
    GENERATION.fetch_add(1, SeqCst) + 1
}
pub fn current(generation: u64) -> bool {
    GENERATION.load(SeqCst) == generation
}
pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
/// Waits for the lock on a thread that must keep answering messages (`pump`) meanwhile.
pub fn lock_pumping(mut pump: impl FnMut()) -> MutexGuard<'static, ()> {
    loop {
        match LOCK.try_lock() {
            Ok(guard) => return guard,
            Err(TryLockError::Poisoned(e)) => return e.into_inner(),
            Err(TryLockError::WouldBlock) => {
                pump();
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
}
/// Runs a UI-thread appearance change: cancels a queued theme apply, waits for a running one.
pub fn exclusive<T>(pump: impl FnMut(), change: impl FnOnce() -> T) -> T {
    claim();
    let _guard = lock_pumping(pump);
    change()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_ui_change_cancels_a_queued_theme_apply_and_waits_for_a_running_one() {
        let queued = claim();
        let running = lock();
        let (sent, received) = std::sync::mpsc::channel();
        let ui = std::thread::spawn(move || exclusive(|| {}, || sent.send(()).unwrap()));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(received.try_recv().is_err(), "ran while a worker held the lock");
        drop(running);
        ui.join().unwrap();
        assert!(received.try_recv().is_ok());
        assert!(!current(queued), "the queued apply must be dropped");
    }
}
