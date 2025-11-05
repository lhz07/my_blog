#[cfg(debug_assertions)]
use std::ops::{Deref, DerefMut};
#[cfg(debug_assertions)]
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(not(debug_assertions))]
pub struct Lock<T>(T);

#[cfg(not(debug_assertions))]
impl<T> Lock<T> {
    pub fn new(inner: T) -> Self {
        Lock(inner)
    }
    #[inline(always)]
    pub fn get(&self) -> &T {
        &self.0
    }
}

#[cfg(debug_assertions)]
pub struct Lock<T>(RwLock<T>);

#[cfg(debug_assertions)]
pub struct ReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
}

#[cfg(debug_assertions)]
pub struct WriteGuard<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
}

#[cfg(debug_assertions)]
impl<T> Lock<T> {
    pub fn new(inner: T) -> Self {
        Lock(RwLock::new(inner))
    }
    pub fn get(&self) -> ReadGuard<'_, T> {
        let guard = self.0.read().unwrap();
        ReadGuard { guard }
    }

    pub fn get_mut(&self) -> WriteGuard<'_, T> {
        let guard = self.0.write().unwrap();
        WriteGuard { guard }
    }
}

#[cfg(debug_assertions)]
impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

#[cfg(debug_assertions)]
impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

#[cfg(debug_assertions)]
impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}
