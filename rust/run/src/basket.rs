#![expect(unreachable_pub)]

use core::fmt::Debug;

use std::sync::{Arc, Mutex, MutexGuard};

pub struct Basket<T>(Arc<Mutex<T>>);

impl<T: Debug> Basket<T> {
    pub fn set(val: T) -> Self {
        Self(Arc::new(Mutex::new(val)))
    }

    pub fn access(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }

    pub fn get(self) -> T {
        Arc::try_unwrap(self.0).unwrap().into_inner().unwrap()
    }
}
