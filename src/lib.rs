#![no_std]

use core::marker::PhantomData;

/// Contains traits that are used in public methods but must not be implemented
/// or used outside of this crate.
mod private;

/// A type that can act as a scope-end handler.
pub trait ScopeEndHandler {
    /// Performs the call.
    fn call_once(self);
}

impl<F: FnOnce()> ScopeEndHandler for F {
    fn call_once(self) {
        (self)()
    }
}

impl<F: FnOnce()> ScopeEndHandler for Option<F> {
    fn call_once(self) {
        if let Some(f) = self {
            (f)()
        }
    }
}

macro_rules! impl_callonce_tuple {
    ($($elem:ident,)*) => {
        impl<$($elem),*> ScopeEndHandler for ($($elem,)*)
        where
            $($elem: ScopeEndHandler,)*
        {
            fn call_once(self) {
                #[allow(non_snake_case)]
                let ($($elem,)*) = self;
                $($elem.call_once();)*
            }
        }
    }
}
impl_callonce_tuple!();
impl_callonce_tuple!(F1,);
impl_callonce_tuple!(F1, F2,);
impl_callonce_tuple!(F1, F2, F3,);
impl_callonce_tuple!(F1, F2, F3, F4,);
impl_callonce_tuple!(F1, F2, F3, F4, F5,);
impl_callonce_tuple!(F1, F2, F3, F4, F5, F6,);

/// A handle of a guard.
///
/// This type is useful because it allows types to take ownership of a guard.
//
// TODO: create 'static handles
pub struct ScopeGuard<'body, 'scope: 'body, F: ScopeEndHandler + 'body> {
    // `None` represents a guard for the `'static` scope.
    inner: Option<&'body mut InnerGuard<'scope, F>>,
}

impl<'body, 'scope, F: ScopeEndHandler> ScopeGuard<'body, 'scope, F> {
    /// Assigns the handler to be called when the scope ends.
    ///
    /// This replaces the existing handler (if one is set).
    ///
    /// Assigning a handler is similar to placing a destructor on the stack,
    /// except that the handler is guaranteed to be run after `'body` ends but
    /// before `'scope` ends (unless the program exits/aborts first or `'scope`
    /// is the `'static` lifetime).
    pub fn assign_handler(&mut self, f: Option<F>) {
        if let Some(ref mut inner) = self.inner {
            inner.assign_handler(f)
        }
    }
}

impl<'body, F: ScopeEndHandler> ScopeGuard<'body, 'static, F> {
    /// Creates a guard for the `'static` lifetime.
    ///
    /// This guard does not contain a handler, so it can be used to protect
    /// only `'static` resources.
    pub fn new_static() -> Self {
        ScopeGuard { inner: None }
    }
}

/// Guard for a scope.
///
/// The lifetime `'scope` is the lifetime of the scope (i.e. the lifetime of
/// resources protected by this `InnerGuard`).
///
/// The guard can optionally contain one handler of type `F`. When the scope
/// ends, the handler is guaranteed to be called (unless the program
/// exits/aborts first). The handler is called within the lifetime `'scope`.
pub struct InnerGuard<'scope, F: ScopeEndHandler> {
    life: PhantomData<&'scope ()>,
    f: Option<F>,
}

impl<'scope, F: ScopeEndHandler> InnerGuard<'scope, F> {
    /// Returns a `ScopeGuard` that wraps the `InnerGuard`.
    ///
    /// This is `unsafe` because only one handle must be created for any
    /// individual `InnerGuard` over its entire lifetime.
    #[doc(hidden)]
    pub unsafe fn wrap(&mut self) -> ScopeGuard<'_, 'scope, F> {
        ScopeGuard { inner: Some(self) }
    }

    /// Assigns the handler to be called when the scope ends.
    ///
    /// This replaces the existing handler (if one is set).
    ///
    /// Assigning a handler is similar to placing a destructor on the stack,
    /// except that the handler is guaranteed to be run (unless the program
    /// exits/aborts first).
    pub fn assign_handler(&mut self, f: Option<F>) {
        self.f = f;
    }

    /// Calls the handler (if there is one) and replaces it with `None`.
    ///
    /// Since the handler is replaced with `None`, subsequent calls are a no-op
    /// unless a new handler is assigned.
    fn call_once(&mut self) {
        if let Some(f) = self.f.take() {
            f.call_once()
        }
    }
}

impl<'scope, F: ScopeEndHandler> Drop for InnerGuard<'scope, F> {
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
/// `InnerGuards<'scope>` is a trait that is implemented by collections of
/// `InnerGuard<'scope>`s. It is implemented for the following types:
///
/// * `InnerGuard<'scope, F> where F: ScopeEndHandler`
/// * Tuples up to length 6:
///   * `()`
///   * `(T1,) where T1: InnerGuards<'scope>`
///   * `(T1, T2) where T1: InnerGuards<'scope>, T2: InnerGuards<'scope>`
///   * …
///   * `(T1, T2, T3, T4, T5, T6) where T1: InnerGuards<'scope>, T2: InnerGuards<'scope>, …`
/// * Arrays up to length 6:
///   * `[T; 0] where T: InnerGuards<'scope>`
///   * `[T; 1] where T: InnerGuards<'scope>`
///   * `[T; 2] where T: InnerGuards<'scope>`
///   * …
///   * `[T; 6] where T: InnerGuards<'scope>`
///
/// Note that even though implementations are provided only for fixed-size
/// collections, it's possible to obtain arbitrarily many `InnerGuard`s by
/// nesting the collections.
// ///
// /// # Example
// ///
// /// ```
// /// use strong_scope_guard::{scope, InnerGuard};
// ///
// /// /// Prevents the slice from being dropped/accessed for the duration of `'scope`.
// /// ///
// /// /// A cleanup closure is called at the end of the scope (e.g. to stop a running DMA request).
// /// fn use_slice<'scope>(slice: &'scope mut [u8], guard: &mut InnerGuard<'scope, fn()>) {
// ///     // Set a closure for cleanup (e.g. to stop a running DMA request).
// ///     // In this example, we're just printing "end of scope".
// ///     guard.assign(Some(|| println!("end of scope")));
// ///     // Do other stuff (e.g. start running the DMA request).
// /// }
// ///
// /// fn main() {
// ///     let mut data = [1, 2, 3];
// ///     scope(
// ///         |guard| {
// ///             use_slice(&mut data, guard);
// ///             // `data` is mutably borrowed for the entire body of this closure.
// ///             // Cleanup code can be placed in the guard's closure.
// ///         }
// ///         // The guard's closure gets called after the body is executed.
// ///     );
// /// }
// /// ```
pub fn scope<'scope, B, G, O>(body: B) -> O
where
    B: FnOnce(&mut G) -> O,
    G: private::InnerGuards<'scope>,
{
    let mut guards = G::new();
    let out = body(&mut guards);
    guards.call_all();
    out
}

/// Defines a guarded scope.
///
/// This macro will no longer be necessary once Rust has generic associated
/// types.
///
/// # Example
///
/// ```rust
/// #[macro_use(scope)]
/// extern crate strong_scope_guard;
///
/// # fn main() {
/// scope!(|a, b| {
///     let z = [1, 2, 3];
///     a.assign_handler(Some(move || { let _ = z; }));
///     b.assign_handler(Some(|| {}));
/// });
/// # }
#[macro_export]
macro_rules! scope {
    (|$($arg:ident),*| $body:expr) => {
        $crate::scope(
            |&mut scope!(@tup_pat (), $($arg),*): &mut scope!(@tup_type (), $($arg),*)| {
                $(
                    #[allow(unused_mut)]
                    #[allow(unsafe_code)]
                    let mut $arg = unsafe { $arg.wrap() };
                )*
                $body
            }
        )
    };
    (|$($arg:ident),*,| $body:expr) => {
        scope!(|$($arg),*| $body)
    };
    (@tup_pat $tup:pat, $arg:ident, $($args:ident),*) => {
        scope!(@tup_pat (ref mut $arg, $tup), $($args),*)
    };
    (@tup_pat $tup:pat, $arg:ident) => {
        (ref mut $arg, $tup)
    };
    (@tup_type $type:ty, $arg:ident, $($args:ident),*) => {
        scope!(@tup_type ($crate::InnerGuard<_>, $type), $($args),*)
    };
    (@tup_type $type:ty, $arg:ident) => {
        ($crate::InnerGuard<_>, $type)
    };
}
