1. **What cost the most.** The two waits for builds: the first ran past the 120 s foreground default, so I ran it a second time in the foreground. Then the gate, which was short. Writing `proved.nika` took three tries. I guessed `str.parse_i64()`, `cli::args().collect()` and a `map` over a `BTreeMap` iterator, and none of them exist in that form. Then the header's `indexes,` carries a comma, which gave a total of 0.
2. **What the brief could have handed me.**
   - The text parse is `text::parse_i64(s)` in `std::text`.
   - Scripts collect their arguments with `for a in cli::args() { args.push(a) }`.
   - A text may only be cut at a position `find` returned (NK1214).
   - `--bounds` and `--optimization` are already global flags, so no new flag on `LowerStd` was needed. Reading clap to find that out took a turn.
   - The exact header format of `bounds_report`.
   - The lowering checks with the full `beside` parses; `without_bodies` is only used for the cache key.
3. **Checks run:** `cargo check`, then one build, two `lower-std --bounds` runs, the script three times, the `--lib sysroot:: bounds_report::` unit tests and the gate (green). Wasted: the second build call. The first one was still running in the background, so the second only waited on it. Also an awk probe for line 63, which found nothing because the site is on line 64 now.
4. **The change that would save the most:** a "Nikaia forms for scripts" cheat sheet in builder-facts. It would cover text to integer, collecting args, iterating a map, sorting tuples, and how to cut text. Second: when the brief knows a commit closes only one step of an issue, it should say so. Then the `Fixes`/`Part` line comes out right the first time, without amending the message.
