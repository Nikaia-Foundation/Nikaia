#!/bin/sh
# Every crate under crates/unsafe/ holds one `unsafe` topic and is checked on
# its own (ADR-218): its tests and lints on stable, and Miri under both of its
# aliasing models on nightly. With arguments, only those crate directories
# (CI checks each crate on its own runner, side by side).
set -e
# Miri's nightly: pinned, since nightly 2026-11-19 ICEs in rustc borrowck on the
# winnow-grammar dependency (task-block `collect`); CI sets it the same way.
NIGHTLY=${NIGHTLY:-nightly-2026-11-18}
[ $# -gt 0 ] || set -- crates/unsafe/*/
for dir in "$@"; do
    echo "== $dir"
    (
        cd "$dir"
        cargo fmt --check
        # Without features and with all of them, since a feature may carry
        # an `unsafe` of its own.
        cargo test --quiet
        cargo test --quiet --all-features
        cargo clippy --quiet --all-targets -- -D warnings
        cargo clippy --quiet --all-features --all-targets -- -D warnings
        cargo +"$NIGHTLY" miri test --quiet --all-features
        MIRIFLAGS=-Zmiri-tree-borrows cargo +"$NIGHTLY" miri test --quiet --all-features
    )
done
