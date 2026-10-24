//! The baton of a crossing into Nikaia (#807): the emitter's walk and the
//! checker's state leave their Rust owner while Nikaia runs. A host method that
//! Nikaia calls and that crosses again finds the slot empty. That is a defect
//! of the host method (it must hand the held value back, as `hosted` does, or
//! take it as a parameter), so it is named: the place of the second crossing
//! and the rule it broke.

/// The value out of an emptied slot, or a panic naming the second crossing.
#[track_caller]
pub(crate) fn out_of<T>(slot: Option<T>, what: &str) -> T {
    match slot {
        Some(value) => value,
        None => {
            let at = std::panic::Location::caller();
            panic!(
                "{what} is already out on a crossing: {at} crosses into Nikaia from \
                 a host method that Nikaia called. The method must hand the value \
                 back (`hosted`) or take it as a parameter; if the sources and the \
                 binary disagree on a host signature, rebuild with `cargo build` \
                 after `lower-std` (#807)"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::out_of;

    #[test]
    fn a_full_slot_gives_its_value() {
        assert_eq!(out_of(Some(3), "the walk"), 3);
    }

    #[test]
    #[should_panic(expected = "the walk is already out on a crossing: ")]
    fn a_nested_crossing_names_itself() {
        let mut slot = Some(1);
        let _outer = slot.take();
        out_of(slot, "the walk");
    }
}
