//! **Whether a key type is text, decided per type when compiling**, and the
//! key's bytes where it is ([ADR-296](../../../docs/specification/adr/adr-296.md)
//! D42).
//!
//! The caller is `nikaia-std`'s map (`map.rs`): one type for every key, which
//! hashes and compares a text key a word at a time instead of through `str`'s
//! `==` (a call of `memcmp`). Rust has no specialisation, so the map asks
//! [`bytes`] for the key's bytes: `Some` for the text types below, `None` for
//! every other type. The answer is a comparison of two type ids, both
//! constants once the call is inlined, so the optimiser keeps one branch and
//! nothing is decided at run time.
//!
//! The type id of a type that may hold a lifetime (`&'a str`) is taken as
//! `dtolnay`'s `typeid` crate takes it: through a trait object whose lifetime
//! is widened to `'static`, which only names the type and touches no value.
//! Lifetimes do not enter a type id, so `&'a str` and `&'static str` are one
//! type here, which is the point. Every `unsafe`, and the argument for it, is
//! listed in `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::sync::Arc;
use core::any::TypeId;
use core::marker::PhantomData;

/// The type id of `T`, lifetimes erased.
#[inline(always)]
pub fn of<T: ?Sized>() -> TypeId {
    trait NonStaticAny {
        fn get_type_id(&self) -> TypeId
        where
            Self: 'static;
    }
    impl<T: ?Sized> NonStaticAny for PhantomData<T> {
        #[inline(always)]
        fn get_type_id(&self) -> TypeId
        where
            Self: 'static,
        {
            TypeId::of::<T>()
        }
    }
    let phantom = PhantomData::<T>;
    let erased: &dyn NonStaticAny = &phantom;
    // SAFETY: only the lifetime bound of the trait object changes; the call
    // reads no value (a `PhantomData` has none) and returns a type id, which
    // holds no reference.
    let widened =
        unsafe { core::mem::transmute::<&dyn NonStaticAny, &'static dyn NonStaticAny>(erased) };
    widened.get_type_id()
}

/// Whether `A` and `B` are one type, lifetimes aside.
#[inline(always)]
pub fn same<A: ?Sized, B: ?Sized>() -> bool {
    of::<A>() == of::<B>()
}

/// Whether `T` is one of the text types [`bytes`] reads.
#[inline(always)]
pub fn is_text<T: ?Sized>() -> bool {
    same::<T, str>()
        || same::<T, &str>()
        || same::<T, String>()
        || same::<T, &String>()
        || same::<T, Box<str>>()
        || same::<T, Rc<str>>()
        || same::<T, Arc<str>>()
        || same::<T, Cow<'_, str>>()
}

/// The bytes of `x` where `T` is text: `str`, `&str`, `String`, `&String`,
/// `Box<str>`, `Rc<str>`, `Arc<str>` or `Cow<str>`. `None` for every other
/// type. They are the bytes `str`'s `Hash` and `==` read, so a hash or a
/// comparison of them is that of the text.
#[inline(always)]
pub fn bytes<T: ?Sized>(x: &T) -> Option<&[u8]> {
    if same::<T, str>() {
        // SAFETY: `T` is `str`, so `&T` is `&str`: the same reference, the
        // same length in its metadata. Never reached for a sized `T`.
        let s = unsafe { core::mem::transmute_copy::<&T, &str>(&x) };
        return Some(s.as_bytes());
    }
    // The rest are sized, so a thin cast keeps the address and the type is
    // the one the id says; only a lifetime inside it may differ, and the
    // bytes handed back live as long as the borrow of `x`.
    let p = x as *const T;
    if same::<T, &str>() {
        // SAFETY: `T` is `&str` up to its lifetime; the `str` it points to
        // outlives the borrow of `x` that the result is tied to.
        return Some(unsafe { *(p as *const &str) }.as_bytes());
    }
    if same::<T, String>() {
        // SAFETY: `T` is `String`.
        return Some(unsafe { &*(p as *const String) }.as_bytes());
    }
    if same::<T, &String>() {
        // SAFETY: `T` is `&String` up to its lifetime, as for `&str`.
        return Some(unsafe { *(p as *const &String) }.as_bytes());
    }
    if same::<T, Box<str>>() {
        // SAFETY: `T` is `Box<str>`.
        return Some(unsafe { &*(p as *const Box<str>) }.as_bytes());
    }
    if same::<T, Rc<str>>() {
        // SAFETY: `T` is `Rc<str>`.
        return Some(unsafe { &*(p as *const Rc<str>) }.as_bytes());
    }
    if same::<T, Arc<str>>() {
        // SAFETY: `T` is `Arc<str>`.
        return Some(unsafe { &*(p as *const Arc<str>) }.as_bytes());
    }
    if same::<T, Cow<'_, str>>() {
        // SAFETY: `T` is `Cow<str>` up to its lifetime, as for `&str`.
        return Some(unsafe { &*(p as *const Cow<'_, str>) }.as_bytes());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn every_text_type_gives_its_bytes() {
        let owned = "Hamburg".to_string();
        let view: &str = &owned;
        assert_eq!(bytes(view), Some(&b"Hamburg"[..]));
        assert_eq!(bytes(&view), Some(&b"Hamburg"[..]));
        assert_eq!(bytes(&owned), Some(&b"Hamburg"[..]));
        assert_eq!(bytes(&&owned), Some(&b"Hamburg"[..]));
        let boxed: Box<str> = view.into();
        assert_eq!(bytes(&boxed), Some(&b"Hamburg"[..]));
        let rc: Rc<str> = view.into();
        assert_eq!(bytes(&rc), Some(&b"Hamburg"[..]));
        let arc: Arc<str> = view.into();
        assert_eq!(bytes(&arc), Some(&b"Hamburg"[..]));
        let cow: Cow<'_, str> = Cow::Borrowed(view);
        assert_eq!(bytes(&cow), Some(&b"Hamburg"[..]));
        assert_eq!(bytes(""), Some(&b""[..]));
    }

    #[test]
    fn other_types_are_not_text() {
        assert_eq!(bytes(&7_i64), None);
        assert_eq!(bytes(&[1_u8, 2][..]), None);
        assert_eq!(bytes(&(1, "a")), None);
        assert_eq!(bytes(&Some("a")), None);
        assert_eq!(bytes(&'a'), None);
        assert!(!is_text::<&&str>());
        assert!(is_text::<&'static str>());
    }

    #[test]
    fn lifetimes_do_not_count() {
        fn local<'a>(_: &'a str) -> bool {
            same::<&'a str, &'static str>()
        }
        let owned = "x".to_string();
        assert!(local(owned.as_str()));
        assert!(!same::<u32, i32>());
    }
}
