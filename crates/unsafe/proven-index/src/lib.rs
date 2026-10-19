//! **An element of a slice read or written without the bounds check**, at a
//! position the caller has proved is inside.
//!
//! The caller is the Nikaia compiler's generated code, at an index its
//! `--optimization=remove-bounds-checks` pass proved: by a loop over the
//! list's own length whose body cannot change that length, or by a
//! certificate of the solver that its checker accepted (ADR-306). Everywhere
//! else the generated code indexes with the check.
//!
//! Every `unsafe` in this crate, and the argument for it, is listed in
//! `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

/// The element at `at`.
///
/// # Safety
///
/// `at < items.len()`. A debug build checks it.
#[inline(always)]
pub unsafe fn read<T>(items: &[T], at: usize) -> &T {
    debug_assert!(
        at < items.len(),
        "a proved index is outside: {at} of {}",
        items.len()
    );
    // SAFETY: the caller guarantees `at < items.len()`.
    unsafe { items.get_unchecked(at) }
}

/// The run from `start` up to `end`, `end` not included.
///
/// # Safety
///
/// `start <= end <= items.len()`. A debug build checks it.
#[inline(always)]
pub unsafe fn cut<T>(items: &[T], start: usize, end: usize) -> &[T] {
    debug_assert!(
        start <= end && end <= items.len(),
        "a proved cut is outside: {start}..{end} of {}",
        items.len()
    );
    // SAFETY: the caller guarantees `start <= end <= items.len()`.
    unsafe { items.get_unchecked(start..end) }
}

/// The `N` elements from `at`, as an array: a word read of bytes (ADR-320
/// D12) is `u64::from_le_bytes(*array::<u8, 8>(bytes, at))`.
///
/// # Safety
///
/// `at <= items.len()` and `N <= items.len() - at`. A debug build checks it.
#[inline(always)]
pub unsafe fn array<T, const N: usize>(items: &[T], at: usize) -> &[T; N] {
    debug_assert!(
        at <= items.len() && N <= items.len() - at,
        "a proved word is outside: {N} from {at} of {}",
        items.len()
    );
    // SAFETY: the caller guarantees the `N` elements from `at` are inside,
    // and `[T; N]` has the layout of `N` consecutive `T`.
    unsafe { &*items.as_ptr().add(at).cast::<[T; N]>() }
}

/// The element at `at`, to change in place.
///
/// # Safety
///
/// `at < items.len()`. A debug build checks it.
#[inline(always)]
pub unsafe fn slot<T>(items: &mut [T], at: usize) -> &mut T {
    debug_assert!(
        at < items.len(),
        "a proved index is outside: {at} of {}",
        items.len()
    );
    // SAFETY: the caller guarantees `at < items.len()`.
    unsafe { items.get_unchecked_mut(at) }
}

/// `value` stored at `at`; the element it replaces is dropped.
///
/// # Safety
///
/// `at < items.len()`. A debug build checks it.
#[inline(always)]
pub unsafe fn write<T>(items: &mut [T], at: usize, value: T) {
    // SAFETY: the caller's guarantee is `slot`'s.
    unsafe { *slot(items, at) = value }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::string::String;
    use std::vec;

    #[test]
    fn reads_the_element_inside() {
        let items = vec![10, 20, 30];
        for at in 0..items.len() {
            // SAFETY: `at` runs over the length.
            assert_eq!(unsafe { *read(&items, at) }, items[at]);
        }
    }

    #[test]
    fn cuts_the_run_inside() {
        let items = vec![10, 20, 30, 40];
        // SAFETY: 1 <= 3 <= 4, and 4 <= 4 <= 4.
        assert_eq!(unsafe { cut(&items, 1, 3) }, &items[1..3]);
        assert!(unsafe { cut(&items, 4, 4) }.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a proved cut is outside")]
    fn a_debug_build_catches_a_wrong_cut() {
        let items = [1, 2];
        // SAFETY: deliberately broken; the debug assertion stops it first.
        let _ = unsafe { cut(&items, 1, 3) };
    }

    #[test]
    fn reads_the_array_inside() {
        let items = [1u8, 2, 3, 4, 5];
        // SAFETY: 1 + 4 <= 5.
        assert_eq!(unsafe { array::<u8, 4>(&items, 1) }, &[2, 3, 4, 5]);
        // SAFETY: 5 + 0 <= 5.
        assert!(unsafe { array::<u8, 0>(&items, 5) }.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a proved word is outside")]
    fn a_debug_build_catches_a_wrong_array() {
        let items = [1u8, 2, 3];
        // SAFETY: deliberately broken; the debug assertion stops it first.
        let _ = unsafe { array::<u8, 4>(&items, 0) };
    }

    #[test]
    fn writes_and_drops_the_element_it_replaces() {
        let mut items = vec![String::from("a"), String::from("b")];
        // SAFETY: 1 < 2.
        unsafe { write(&mut items, 1, String::from("c")) };
        assert_eq!(items, ["a", "c"]);
        // SAFETY: 0 < 2.
        unsafe { slot(&mut items, 0).push('!') };
        assert_eq!(items, ["a!", "c"]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a proved index is outside")]
    fn a_debug_build_catches_a_wrong_proof() {
        let items = [1, 2];
        // SAFETY: deliberately broken; the debug assertion stops it first.
        let _ = unsafe { read(&items, 2) };
    }
}
