use core::marker::PhantomData;
use super::{CallOnce, Guard};

/// Represents a collection of `Guard`s.
///
/// For an implementation of this trait to be safe, in its `call_all()`
/// implementation, it must call `call_all()` on all children it creates in
/// `new()`. In other words, it must not be possible to create a child in
/// `new()` that is leaked while this collection lives.
pub unsafe trait Guards<'a> {
    /// Constructs a new instance of this type.
    ///
    /// This method must not be able to be called outside of the `scope()`
    /// function because that would allow creation of `Guard` instances
    /// with arbitrary `'a` lifetimes.
    fn new() -> Self;

    /// Calls `.call_all()` all of the children of this collection.
    ///
    /// This method must not be able to be called outside of this crate because
    /// that would allow the guards to be called early (before the body of the
    /// scope is finished).
    fn call_all(&mut self);
}

macro_rules! impl_tuple {
    ($($elem:ident,)*) => {
        unsafe impl<'a, $($elem),*> Guards<'a> for ($($elem,)*)
        where
            $($elem: Guards<'a>,)*
        {
            fn new() -> Self {
                ($($elem::new(),)*)
            }

            #[allow(non_snake_case)]
            fn call_all(&mut self) {
                let &mut ($(ref mut $elem,)*) = self;
                $($elem.call_all();)*
            }
        }
    }
}

impl_tuple!();
impl_tuple!(T1,);
impl_tuple!(T1, T2,);
impl_tuple!(T1, T2, T3,);
impl_tuple!(T1, T2, T3, T4,);
impl_tuple!(T1, T2, T3, T4, T5,);
impl_tuple!(T1, T2, T3, T4, T5, T6,);

macro_rules! impl_array {
    ($len:expr, [$($elem:ident),*]) => {
        unsafe impl<'a, T> Guards<'a> for [T; $len]
        where
            T: Guards<'a>,
        {
            fn new() -> Self {
                [$($elem::new()),*]
            }

            fn call_all(&mut self) {
                for elem in self {
                    elem.call_all()
                }
            }
        }
    }
}

impl_array!(0, []);
impl_array!(1, [T]);
impl_array!(2, [T, T]);
impl_array!(3, [T, T, T]);
impl_array!(4, [T, T, T, T]);
impl_array!(5, [T, T, T, T, T]);
impl_array!(6, [T, T, T, T, T, T]);

unsafe impl<'a, F: CallOnce> Guards<'a> for Guard<'a, F> {
    fn new() -> Self {
        Guard {
            life: PhantomData,
            f: None,
        }
    }

    fn call_all(&mut self) {
        self.call_once()
    }
}
