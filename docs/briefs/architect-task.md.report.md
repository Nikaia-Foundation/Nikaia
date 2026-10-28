Result: done
Batch: b905-variant-list-part.md, p2-large-files-plan.md
Owner: none
Notes: cut at n903b on Found (#903 done, ee19152; Changelog line in its report, version raise owed). Found widened by probes on 42c0246 and filed as #905 (ready): a variant's Vec[String] part bound in a match over an owned value is typed Vec[ref String] (return, hand-on, for refused; let-bound match reaches rustc); in tools a drained part lowers to an unused `let mut`. RED was the empty state file; CI on 00451bf..42c0246 running. Next after the batch: brief #557.
