1. **Most turns:** finding the cause, about 15 turns. I read `expect_type`, `Ty::fits`, `named_fits`, `widened_into` and `check_widened_slot`, and none of them was it. What found it was a set of small probes: a user struct was refused, `std` types were accepted, and `u32` was refused. That pointed to the open-number rule `no_number_is` in `check_values.nika`. I should have run those probes first. The test round I lost was my own doing: my `async` tests hit NK2110 because they had no statement beside the `async`.
2. **What the brief could have handed me:**
   - That an open integer literal has no type until a use decides it (ADR-285 D23), so it reads as "unknown" and passes `fits`.
   - The name `no_number_is` / `no_integer_is` ("a library's type is not claimed").
   - That `async(within:)` needs a statement beside it (NK2110), or simply the path of an existing test to copy, `async_within_value_in_time.nika`.
3. **Checks run:**
   - Useful: old- and new-binary probes, one build and `lower-std` cycle, the filtered `tests/compiler` runs, the old/new `check` comparison over `examples/` and `packages/`, clippy, fmt, pre-push.
   - Partly wasted: the first `a_duration` bless run, which picked up a test I was still editing, and the re-run of `async_parse_fmt_formats_every_form`, which fails whatever binary runs it.
4. **One change that would save the most:** in builder.md, *for a "checker accepts X" defect, first probe the neighbouring shapes (user type, std type, literal, typed variable, open variable) on the old binary, before reading code.* Close second: a trap line in builder-facts saying that open numbers are typed by `no_number_is`.
