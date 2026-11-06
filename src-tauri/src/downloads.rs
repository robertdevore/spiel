//! Exclusive ownership of model downloads and idle model-store maintenance.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

struct ActiveDownload {
    model_id: String,
    cancel: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
pub struct Downloads {
    active: Arc<Mutex<Option<ActiveDownload>>>,
}

pub struct Reservation {
    downloads: Downloads,
    cancel: Arc<AtomicBool>,
}

impl Downloads {
    pub fn reserve(&self, model_id: &str) -> Option<Reservation> {
        let mut active = self.active.lock().unwrap();
        if active.is_some() {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *active = Some(ActiveDownload {
            model_id: model_id.into(),
            cancel: cancel.clone(),
        });
        Some(Reservation {
            downloads: self.clone(),
            cancel,
        })
    }

    pub fn is_active(&self) -> bool {
        self.active.lock().unwrap().is_some()
    }

    pub fn is_downloading(&self, model_id: &str) -> bool {
        self.active
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|active| active.model_id == model_id)
    }

    pub fn cancel(&self) {
        if let Some(active) = self.active.lock().unwrap().as_ref() {
            active.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn when_idle<T>(&self, maintenance: impl FnOnce() -> T) -> Option<T> {
        let active = self.active.lock().unwrap();
        if active.is_some() {
            return None;
        }
        let result = maintenance();
        drop(active);
        Some(result)
    }
}

impl Reservation {
    pub fn is_canceled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        *self.downloads.active.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Barrier};

    #[test]
    fn competing_downloads_have_one_owner_until_release() {
        let downloads = Downloads::default();
        let start = Arc::new(Barrier::new(3));
        let finish = Arc::new(Barrier::new(3));
        let (tx, rx) = mpsc::channel();
        let threads: Vec<_> = ["tiny", "base"]
            .into_iter()
            .map(|model| {
                let downloads = downloads.clone();
                let start = start.clone();
                let finish = finish.clone();
                let tx = tx.clone();
                std::thread::spawn(move || {
                    start.wait();
                    let reservation = downloads.reserve(model);
                    tx.send(reservation.is_some()).unwrap();
                    finish.wait();
                    drop(reservation);
                })
            })
            .collect();
        start.wait();
        let owners = usize::from(rx.recv().unwrap()) + usize::from(rx.recv().unwrap());
        assert_eq!(owners, 1);
        assert!(downloads.is_active());
        assert_eq!(
            downloads.when_idle(|| panic!("must not clean active download")),
            None::<()>
        );
        finish.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(!downloads.is_active());
        assert_eq!(downloads.when_idle(|| 42), Some(42));
    }

    #[test]
    fn preflight_failure_releases_slot_and_new_download_has_fresh_cancellation() {
        fn rejected_preflight(downloads: &Downloads) -> Result<(), ()> {
            let _reservation = downloads.reserve("tiny").unwrap();
            Err(())
        }
        let downloads = Downloads::default();
        assert!(rejected_preflight(&downloads).is_err());
        assert!(!downloads.is_active());
        let reservation = downloads.reserve("base").unwrap();
        assert!(downloads.is_downloading("base"));
        assert!(!downloads.is_downloading("tiny"));
        downloads.cancel();
        assert!(reservation.is_canceled());
        drop(reservation);
        let next = downloads.reserve("tiny").unwrap();
        assert!(!next.is_canceled());
    }

    #[test]
    fn maintenance_holds_exclusion_until_its_work_finishes() {
        let downloads = Downloads::default();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        let worker = downloads.clone();
        let maintenance = std::thread::spawn(move || {
            worker.when_idle(|| {
                entered_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
            })
        });
        entered_rx.recv().unwrap();
        assert!(downloads.active.try_lock().is_err());
        finish_tx.send(()).unwrap();
        assert_eq!(maintenance.join().unwrap(), Some(()));
        assert!(downloads.reserve("tiny").is_some());
    }
}
