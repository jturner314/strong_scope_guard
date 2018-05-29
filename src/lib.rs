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

/// A guard that protects access by parallel tasks to temporary resources.
pub trait ScopeGuard<'scope>: private::ScopeGuardPriv<'scope> {
    type Handler: ScopeEndHandler;

    fn handler_mut(&mut self) -> Option<&mut Self::Handler>;

    /// Convenience method for assigning the handler.
    fn set_handler(&mut self, handler: Self::Handler) {
        if let Some(h) = self.handler_mut() {
            *h = handler
        }
    }
}

/// A guard that protects non-`'static` data.
#[derive(Debug)]
pub struct LocalScopeGuard<'scope, H: ScopeEndHandler> {
    scope: PhantomData<&'scope ()>,
    handler: H,
}

impl<'scope, H: ScopeEndHandler> ScopeGuard<'scope> for LocalScopeGuard<'scope, H> {
    type Handler = H;

    fn handler_mut(&mut self) -> Option<&mut H> {
        Some(&mut self.handler)
    }
}

impl<'scope, H: ScopeEndHandler> private::ScopeGuardPriv<'scope> for LocalScopeGuard<'scope, H> {
    fn new() -> Self {
        LocalScopeGuard {
            scope: PhantomData,
            handler: H::none(),
        }
    }

    fn call(&mut self) {
        mem::replace(&mut self.handler, H::none()).call()
    }
}

impl<'scope, H: ScopeEndHandler> Drop for LocalScopeGuard<'scope, H> {
    // This drop implementation is necessary in case of panics.
    fn drop(&mut self) {
        private::ScopeGuardPriv::call(self)
    }
}

/// A guard that protects `'static` data.
#[derive(Debug)]
pub struct StaticScopeGuard<H: ScopeEndHandler> {
    handler: PhantomData<H>,
}

impl<H: ScopeEndHandler> StaticScopeGuard<H> {
    /// Returns a new `StaticScopeGuard`.
    pub fn new() -> Self {
        StaticScopeGuard {
            handler: PhantomData,
        }
    }
}

impl<H: ScopeEndHandler> ScopeGuard<'static> for StaticScopeGuard<H> {
    type Handler = H;

    fn handler_mut(&mut self) -> Option<&mut H> {
        None
    }
}

impl<H: ScopeEndHandler> private::ScopeGuardPriv<'static> for StaticScopeGuard<H> {
    fn new() -> Self {
        Self::new()
    }

    fn call(&mut self) {}
}

/// Creates a new scope.
///
/// This provides support for what are effectively destructors that are
/// *guaranteed to be run* when the scope ends (unless the program exits/aborts
/// first).
///
/// `LocalScopeGuards` is a trait that is implemented by collections of
/// `LocalGuard`s. It is implemented for the following types:
///
/// * `LocalScopeGuardGuard<'scope, H> where H: ScopeEndHandler`
/// * Tuples up to length 6:
///   * `()`
///   * `(T1,) where T1: LocalScopeGuards<'scope>`
///   * `(T1, T2) where T1: LocalScopeGuards<'scope>, T2: LocalScopeGuards<'scope>`
///   * …
///   * `(T1, T2, T3, T4, T5, T6) where T1: LocalScopeGuards<'scope>, T2: LocalScopeGuards<'scope>, …`
/// * Arrays up to length 6:
///   * `[T; 0] where T: LocalScopeGuards<'scope>`
///   * `[T; 1] where T: LocalScopeGuards<'scope>`
///   * `[T; 2] where T: LocalScopeGuards<'scope>`
///   * …
///   * `[T; 6] where T: LocalScopeGuards<'scope>`
///
/// Note that even though implementations are provided only for fixed-size
/// collections, it's possible to obtain arbitrarily many `LocalScopeGuard`s by
/// nesting the collections.
// ///
// /// # Example
// ///
// /// ```
// /// use strong_scope_guard::{scope, LocalScopeGuard, ScopeGuard};
// ///
// /// /// Prevents the slice from being dropped/accessed for the duration of `'scope`.
// /// ///
// /// /// A cleanup closure is called at the end of the scope (e.g. to stop a running DMA request).
// /// fn use_slice<'scope, G>(slice: &'scope mut [u8], mut guard: G)
// /// where
// ///     G: ScopeGuard<'scope, Handler = Option<fn()>>,
// /// {
// ///     // Set a closure for cleanup (e.g. to stop a running DMA request).
// ///     // In this example, we're just printing "end of scope".
// ///     guard.set_handler(Some(|| println!("end of scope")));
// ///     // Do other stuff (e.g. start running the DMA request).
// /// }
// ///
// /// fn main() {
// ///     let mut data = [1, 2, 3];
// ///     scope(
// ///         |guard: LocalScopeGuard<_>| {
// ///             use_slice(&mut data, guard);
// ///             // `data` is mutably borrowed for the entire body of this closure.
// ///             // Cleanup code can be placed in the guard's closure.
// ///             (guard, ())
// ///         }
// ///         // The guard's closure gets called after the body is executed.
// ///     );
// /// }
// /// ```
pub fn scope<'scope, B, G, O>(body: B) -> O
where
    B: FnOnce(G) -> (G, O),
    G: private::LocalScopeGuards<'scope>,
{
    let guards = G::new();
    let (mut guards, out) = body(guards);
    guards.call_all();
    out
}

/// Convenience macro for creating a guarded scope.
///
/// This macro makes it easy to generate arbitrarily many guards, and it helps
/// the compiler's type inference in most cases.
///
/// # Example
///
/// The macro in this example
///
/// ```rust
/// #[macro_use(scope)]
/// extern crate strong_scope_guard;
///
/// use strong_scope_guard::ScopeGuard;
///
/// # fn main() {
/// scope!(|(a, b)| {
///     let z = [1, 2, 3];
///     a.set_handler(Some(move || { let _ = z; }));
///     b.set_handler(Some(|| {}));
///     ((a, b), ())
/// });
/// # }
/// ```
///
/// expands to (simplifying the paths to `scope` and `LocalScopeGuard`):
///
/// ```ignore
/// scope(|(((), a), b): (((), LocalScopeGuard<_>), LocalScopeGuard<_>)| {
///     #[allow(unused_mut)]
///     let (mut a, mut b) = (a, b);
///     let ((a, b), out) = {
///         let z = [1, 2, 3];
///         a.set_handler(Some(move || { let _ = z; }));
///         b.set_handler(Some(|| { }));
///         ((a, b), ())
///     };
///     ((((), a), b), out)
/// });
/// ```
#[macro_export]
macro_rules! scope {
    (|($($arg:ident),*)| $body:expr) => {
        $crate::scope(scope!(@closure |($($arg),*)| $body))
    };
    (|($($arg:ident),*,)| $body:expr) => {
        scope!(|($($arg),*)| $body)
    };
    (move |($($arg:ident),*)| $body:expr) => {
        $crate::scope(move scope!(@closure |($($arg),*)| $body))
    };
    (move |($($arg:ident),*,)| $body:expr) => {
        scope!(move |($($arg),*)| $body)
    };
    (@closure |($($arg:ident),*)| $body:expr) => {
        |scope!(@nest_pat (), $($arg),*): scope!(@nest_type (), $($arg),*)| {
            #[allow(unused_mut)]
            let ($(mut $arg),*) = ($($arg),*);
            let (($($arg),*), out) = $body;
            (scope!(@nest_expr (), $($arg),*), out)
        }
    };
    (@nest_pat $tup:pat, $arg:ident, $($args:ident),*) => {
        scope!(@nest_pat ($tup, $arg), $($args),*)
    };
    (@nest_pat $tup:pat, $arg:ident) => {
        ($tup, $arg)
    };
    (@nest_type $type:ty, $arg:ident, $($args:ident),*) => {
        scope!(@nest_type ($type, $crate::LocalScopeGuard<_>), $($args),*)
    };
    (@nest_type $type:ty, $arg:ident) => {
        ($type, $crate::LocalScopeGuard<_>)
    };
    (@nest_expr $tup:expr, $arg:ident, $($args:ident),*) => {
        scope!(@nest_expr ($tup, $arg), $($args),*)
    };
    (@nest_expr $tup:expr, $arg:ident) => {
        ($tup, $arg)
    };
}
