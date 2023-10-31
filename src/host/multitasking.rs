use futures::{future::pending, FutureExt};
use slab::Slab;
// use std::sync::Mutex;
use std::{sync::Arc, time::Instant};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Debug, Default)]
pub struct HostMutex {
    inner: Arc<Mutex<()>>,
    lock: Option<OwnedMutexGuard<()>>,
}

#[derive(Debug, Default)]
pub struct MutexPool {
    mutexes: Slab<HostMutex>,
}

impl MutexPool {
    /// Creates a mutex, returning its ID.
    pub fn create_mutex(&mut self) -> usize {
        self.mutexes.insert(HostMutex::default())
    }
    /// Creates a mutex, returning its ID.
    pub fn delete_mutex(&mut self, mutex_id: usize) {
        self.mutexes.remove(mutex_id);
    }

    /// Locks a mutex by ID, cancelling on timeout, and returning a boolean of whether the lock was
    /// successful.
    pub async fn lock(&mut self, mutex_id: usize, timeout: Option<Instant>) -> bool {
        let sleep = timeout.map_or_else(
            || pending().boxed(),
            |i| tokio::time::sleep_until(i.into()).boxed(),
        );

        let mutex = self.mutexes.get_mut(mutex_id).unwrap();
        tokio::select! {
            biased;
            lock = mutex.inner.clone().lock_owned() => {
                mutex.lock = Some(lock);
                true
            }
            _ = sleep => false,
        }
    }

    pub fn unlock(&mut self, mutex_id: usize) {
        let mutex = self.mutexes.get_mut(mutex_id).unwrap();
        mutex.lock.take().unwrap();
    }
}
