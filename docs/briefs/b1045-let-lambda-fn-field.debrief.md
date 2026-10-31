1. **Most cost:** the time went to the gate: `tests/compiler` at both settings, the guard set and clippy, in one background run. That was needed, because I changed a `tools/check_*.nika` file. The turns went to row b. After the fix it hit NK1245 instead of running, and I needed two extra probes (a `let k = h.f` variant on the old binary) to show the refusal was already there on the unfixed compiler. Three more turns went to getting a throwing-function probe to parse: `throws String`, then `?` in a lambda, then an `Error` enum.

2. **Looked up:**
   - How a fallible function is written (an `enum` that implements `Error`, `throws` with no type, no `?` inside a lambda). I copied it from an existing NK2605 test.
   - That an untyped lambda parameter can't take a type annotation (`fn(x: i64)` doesn't parse).
   - That #997's typing by first use doesn't reach a struct field. The brief's cause section hinted at this but didn't connect it to row b.

   The locations, the cause and the line numbers in the brief were exact and saved all the code reading.

3. **Checks:**
   - Run: rows a–e on the old binary, then on the fixed one; one `lower-std` build cycle (fixpoint clean); `where.nika`, which listed no tests; the gate; `cargo fmt --check`; `pre-push-check.sh`.
   - Partly wasted: the two broken throwing-function probes, and copying the binary for the gate run (harmless).
   - Not run: crate unit tests, since I changed no Rust source.

4. **One change:** the brief's rows should be probed past their first refusal, for example with the field call written as `let k = h.f; k(1)`. Then row b's separate NK1245 would have been in the brief, and the test would have been specified without it. A smaller help: a five-line template in `builder-facts.md` for a fallible function in a probe.
