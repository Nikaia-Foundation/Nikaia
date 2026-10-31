1. **What cost the most:** the package test runs. Each `nikaia test --project packages/yaml` takes about 3–4 minutes, and I ran it twice: once for the 14 old tests, once more with `--bless` for the new one. The `it` build for `fmt::every_nika_file` added about a minute. Two of my own sed slips cost turns too: the first `rest.drain()` replacement also hit the Seq/Map loops, and one line number was off by one.
2. **What I looked up that the brief could have given me:**
   - That a `for` over an owned `Vec` only borrows, and taking elements away is written `for x in xs.drain()` (Part I 6.5). This form is not in builder-facts' syntax list.
   - Whether `nikaia.contracts` and `nikaia.derived` belong in a package commit. I answered it from `git log`.
   - Whether the tree keeps a scalar's text as written for Null and Bool. A one-line probe answered it. The architect's own probe of this step should have caught that the tree drops it, since that is what blocked steps 2–3.
3. **Checks:**
   - Ran: y22's Probe A, `nikaia check`, `nikaia fmt`, the package tests twice, `fmt::every_nika_file`, `pre-push-check.sh`, and the gap probe.
   - Partly wasted: the first test run. It started before I added the new test file, so the second run had to repeat everything. Writing the test first would have saved one whole run.
   - Correctly skipped: clippy and the cargo build, since nothing in `crates/` changed.
4. **One change that would have saved the most:** the brief's pre-flight should have checked that the tree keeps everything an event line needs, by parsing `[~, NULL, True, !!int "1"]`, and settled where that text lives before the run. Steps 2–3 then would not have been blocked. For builder-facts: "a package commit includes `nikaia.contracts` and `nikaia.derived`, never `nikaia.lock` or a `nikaia.proofs` you did not change", and "a consuming loop is `for x in v.drain()`".
