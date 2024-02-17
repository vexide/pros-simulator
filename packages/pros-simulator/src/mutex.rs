use std::{
    ops::{Deref, DerefMut},
    panic::Location,
    sync::{Arc, Mutex as StdMutex, Weak},
};

use futures_util::Future;
use snafu::{ResultExt, Snafu};
use tokio::sync::{Mutex as TokioMutex, MutexGuard as TokioMutexGuard};

pub type Mutex<T> = DebuggableMutex<T>;
pub type MutexGuard<'a, T> = DebuggableMutexGuard<'a, T>;

#[derive(Snafu, Debug)]
#[snafu(display("Lock failed at {lock_location:?}: {source}"))]
pub struct TryLockError {
    lock_location: Option<String>,
    source: tokio::sync::TryLockError,
}

pub struct DebuggableMutex<T> {
    inner: TokioMutex<T>,
    last_lock_location: Arc<StdMutex<Option<String>>>,
}

impl<T> Deref for DebuggableMutex<T> {
    type Target = TokioMutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for DebuggableMutex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> DebuggableMutex<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: TokioMutex::new(inner),
            last_lock_location: Default::default(),
        }
    }

    #[track_caller]
    pub fn try_lock(&self) -> Result<DebuggableMutexGuard<T>, TryLockError> {
        let lock_location = {
            let mut location = self.last_lock_location.lock().unwrap();
            std::mem::replace(&mut *location, Some(Location::caller().to_string()))
        };
        self.inner
            .try_lock()
            .map(|guard| DebuggableMutexGuard {
                guard,
                lock_location: Arc::downgrade(&self.last_lock_location),
            })
            .context(TryLockSnafu { lock_location })
    }

    #[track_caller]
    pub fn lock(&self) -> impl Future<Output = DebuggableMutexGuard<T>> {
        let caller = Location::caller().to_string();
        async move {
            let guard = self.inner.lock().await;
            let mut location = self.last_lock_location.lock().unwrap();
            *location = Some(caller);
            DebuggableMutexGuard {
                guard,
                lock_location: Arc::downgrade(&self.last_lock_location),
            }
        }
    }
}

pub struct DebuggableMutexGuard<'a, T> {
    guard: TokioMutexGuard<'a, T>,
    lock_location: Weak<StdMutex<Option<String>>>,
}

impl<'a, T> Deref for DebuggableMutexGuard<'a, T> {
    type Target = TokioMutexGuard<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for DebuggableMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<'a, T> Drop for DebuggableMutexGuard<'a, T> {
    fn drop(&mut self) {
        // clear the lock location
        if let Some(location) = self.lock_location.upgrade() {
            let mut location = location.lock().unwrap();
            *location = None;
        }
    }
}
