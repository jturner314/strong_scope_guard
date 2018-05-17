#![no_std]

use core::marker::PhantomData;

/// Contains traits that are used in public methods but must not be implemented
/// or used outside of this crate.
mod private;

/// A trait equivalent to [`FnOnce()`][`FnOnce`].
///
/// This is useful because it's possible to implement this trait for your own
/// types. (This is not true for [`FnOnce`].)
///
/// [`FnOnce`]: https://doc.rust-lang.org/stable/core/ops/trait.FnOnce.html
pub trait CallOnce {
    /// Performs the call.
    fn call_once(self);
}

impl<F: FnOnce()> CallOnce for F {
    fn call_once(self) {
        (self)()
    }
}

/// Guard for a scope.
///
/// The lifetime `'a` is the lifetime of the scope (i.e. the lifetime of
/// resources protected by this `ScopeGuard`).
///
/// The guard can optionally contain one closure of type `F`. When the scope
/// ends, the closure is guaranteed to be called (unless the program
/// exits/aborts first). The closure is called within the lifetime `'a`.
pub struct ScopeGuard<'a, F: CallOnce> {
    life: PhantomData<&'a ()>,
    f: Option<F>,
}

impl<F: CallOnce> ScopeGuard<'static, F> {
    /// Creates a guard for the `'static` lifetime.
    ///
    /// This guard's closure will be called only if the guard is dropped. This
    /// should never be an issue in practice because a `ScopeGuard<'static, F>`
    /// can protect only `'static` resources.
    pub fn new_static() -> ScopeGuard<'static, F> {
        ScopeGuard {
            life: PhantomData,
            f: None,
        }
    }
}

impl<'a, F: CallOnce> ScopeGuard<'a, F> {
    /// Assigns the closure to be called when the scope ends.
    ///
    /// This replaces the existing closure (if one is set).
    ///
    /// Assigning a closure is similar to placing a destructor on the stack,
    /// except that the closure is guaranteed to be run (unless the program
    /// exits/aborts first).
    pub fn assign(&mut self, f: Option<F>) {
        self.f = f;
    }

    /// Calls the closure (if there is one) and replaces it with `None`.
    ///
    /// Since the closure is replaced with `None`, subsequent calls are a no-op
    /// unless a new closure is assigned.
    fn call_once(&mut self) {
        if let Some(f) = self.f.take() {
            f.call_once()
        }
    }
}

impl<'a, F: CallOnce> Drop for ScopeGuard<'a, F> {
    // This drop implementation is necessary in case of panics.
    fn drop(&mut self) {
        self.call_once()
    }
}

/// Creates a new scope.
///
/// This provides support for what are effectively destructors that are
/// *guaranteed to be run* when the scope ends (unless the program exits/aborts
/// first).
///
/// `ScopeGuards<'a>` is a trait that is implemented by collections of
/// `ScopeGuard<'a>`s. It is implemented for the following types:
///
/// * `ScopeGuard<'a, F> where F: CallOnce`
/// * Tuples up to length 6:
///   * `()`
///   * `(T1,) where T1: ScopeGuards<'a>`
///   * `(T1, T2) where T1: ScopeGuards<'a>, T2: ScopeGuards<'a>`
///   * …
///   * `(T1, T2, T3, T4, T5, T6) where T1: ScopeGuards<'a>, T2: ScopeGuards<'a>, …`
/// * Arrays up to length 6:
///   * `[T; 0] where T: ScopeGuards<'a>`
///   * `[T; 1] where T: ScopeGuards<'a>`
///   * `[T; 2] where T: ScopeGuards<'a>`
///   * …
///   * `[T; 6] where T: ScopeGuards<'a>`
///
/// Note that even though implementations are provided only for fixed-size
/// collections, it's possible to obtain arbitrarily many `ScopeGuard`s by
/// nesting the collections.
///
/// # Example
///
/// ```
/// use strong_scope_guard::{scope, ScopeGuard};
///
/// /// Prevents the slice from being dropped/accessed for the duration of `'a`.
/// ///
/// /// A cleanup closure is called at the end of the scope (e.g. to stop a running DMA request).
/// fn use_slice<'a>(slice: &'a mut [u8], guard: &mut ScopeGuard<'a, fn()>) {
///     // Set a closure for cleanup (e.g. to stop a running DMA request).
///     // In this example, we're just printing "end of scope".
///     guard.assign(Some(|| println!("end of scope")));
///     // Do other stuff (e.g. start running the DMA request).
/// }
///
/// fn main() {
///     let mut data = [1, 2, 3];
///     scope(
///         |guard| {
///             use_slice(&mut data, guard);
///             // `data` is mutably borrowed for the entire body of this closure.
///             // Cleanup code can be placed in the guard's closure.
///         }
///         // The guard's closure gets called after the body is executed.
///     );
/// }
/// ```
pub fn scope<'a, B, G, O>(body: B) -> O
where
    B: FnOnce(&mut G) -> O,
    G: private::ScopeGuards<'a>,
{
    let mut guards = G::new();
    let out = body(&mut guards);
    guards.call_all();
    out
}
