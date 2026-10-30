Result: done
Files: crates/nikaia-std/std.contracts tests/compiler/tests/a_set_is_walked_by_iter.nika tests/compiler/tests/a_set_is_walked_by_iter.stdout
Commit: std: every set has iter() (ADR-collections-2 D3)
Body: Only the contract was needed: `collections::BTreeSet::iter` (`Seq[ref(self) T] sync ends sized`, smallest first) and `collections::HashSet::iter` (`Seq[ref(self) T] sync sized`, no order) in std.contracts; the emitter already lowers both to Rust's `.iter()` (no checker or lowering row, `lower-std` fixpoint without diff), in a program and in a tools file. The orphan comment above `BTreeSet::first` moved onto `BTreeSet::clone`. The test's HashSet sum is bound with `let total: i64` first, because `println(v.iter().sum())` fails at rustc for a Vec too (see Found).
Fixes: #1006
Tests: a_set_is_walked_by_iter (tests/compiler 358 passed; NK1171 on the unfixed compiler); K3a form lowered to `named.iter().any(|name| ...)` and run through rustc; tools-file probe under lower-std, restored
Gate: green
Deviation: none
Found: `println(v.iter().sum())` on a `Vec[i64]` reaches rustc: "type annotations needed, Sum<i64>" (probe: `let v: Vec[i64] = [3, 4]` then `println(v.iter().sum())`); `let t: i64 = v.iter().sum()` works
Needs: none
Changelog: `std` gains `iter()` on `collections::BTreeSet` (smallest first) and `collections::HashSet` (no order), lending the elements one at a time.
Owner: none
Disk freed: 0 GB (no clone, no own target)
Rs2nika: none
Pushed: d2986cd9545f275f051b2e79d4cdd00be9f54de5
