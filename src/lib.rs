#![no_std]

use core::marker::PhantomData;
use core::mem;

/// Contains traits that are used in public methods but must not be implemented
/// or used outside of this crate.
mod private;

/// A type that can act as a scope-end handler.
pub trait ScopeEndHandler {
    /// Returns a no-op handler.
    fn none() -> Self;

    /// Executes the handler.
    fn call(self);
}

impl<F: FnOnce()> ScopeEndHandler for Option<F> {
    fn none() -> Self {
        None
    }

    fn call(self) {
        if let Some(f) = self {
            (f)()
        }
    }
}

macro_rules! impl_scopeendhandler_tuple {
    ($($elem:ident,)*) => {
        impl<$($elem),*> ScopeEndHandler for ($($elem,)*)
        where
            $($elem: ScopeEndHandler,)*
        {
            fn none() -> Self {
                ($($elem::none(),)*)
            }

            fn call(self) {
                #[allow(non_snake_case)]
                let ($($elem,)*) = self;
                $($elem.call();)*
            }
         }
     }
 }
impl_scopeendhandler_tuple!();
impl_scopeendhandler_tuple!(H1,);
impl_scopeendhandler_tuple!(H1, H2,);
impl_scopeendhandler_tuple!(H1, H2, H3,);
impl_scopeendhandler_tuple!(H1, H2, H3, H4,);
impl_scopeendhandler_tuple!(H1, H2, H3, H4, H5,);
impl_scopeendhandler_tuple!(H1, H2, H3, H4, H5, H6,);

/// A handle of a guard.
///
/// This type is useful because it allows types to take ownership of a guard.
#[derive(Debug)]
pub struct ScopeGuard<'body, 'scope: 'body, H: ScopeEndHandler + 'body> {
    // `None` represents a guard for the `'static` scope.
    inner: Option<&'body mut InnerGuard<'scope, H>>,
}

impl<'body, 'scope, H: ScopeEndHandler> ScopeGuard<'body, 'scope, H> {
    /// Returns a mutable reference to the handler.
    ///
    /// If this guard is for the `'static` lifetime, returns `None` since
    /// `'static` guards do not contain a handler.
    pub fn handler_mut(&mut self) -> Option<&mut H> {
        self.inner.as_mut().map(|inner| &mut inner.handler)
    }

    /// Convenience method for assigning the handler.
    pub fn set_handler(&mut self, handler: H) {
        if let Some(ref mut inner) = self.inner {
            inner.handler = handler;
        }
    }
}

impl<'body, H: ScopeEndHandler> ScopeGuard<'body, 'static, H> {
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
/// The guard contains one handler of type `H`. When the scope ends, the
/// handler is guaranteed to be called (unless the program exits/aborts first).
/// The handler is called within the lifetime `'scope`.
#[derive(Debug)]
pub struct InnerGuard<'scope, H: ScopeEndHandler> {
    life: PhantomData<&'scope ()>,
    handler: H,
}

impl<'scope, H: ScopeEndHandler> InnerGuard<'scope, H> {
    /// Returns a `ScopeGuard` that wraps the `InnerGuard`.
    ///
    /// This is `unsafe` because only one handle must be created for any
    /// individual `InnerGuard` over its entire lifetime.
    #[doc(hidden)]
    pub unsafe fn wrap(&mut self) -> ScopeGuard<'_, 'scope, H> {
        ScopeGuard { inner: Some(self) }
    }

    /// Calls the handler and replaces it with `H::none()`.
    ///
    /// Since the handler is replaced with `H::none()`, subsequent calls are a
    /// no-op unless a new handler is set.
    fn call(&mut self) {
        mem::replace(&mut self.handler, H::none()).call()
    }
}

impl<'scope, H: ScopeEndHandler> Drop for InnerGuard<'scope, H> {
    // This drop implementation is necessary in case of panics.
    fn drop(&mut self) {
        self.call()
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
/// * `InnerGuard<'scope, H> where H: ScopeEndHandler`
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
///     a.set_handler(Some(move || { let _ = z; }));
///     b.set_handler(Some(|| {}));
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
