Result: done
Batch: b15-1060-trusted-map-name-collision.md b13-1057-field-of-a-coalesce-and-let.md b14-947-get-or-return-compared.md
Owner: none
Notes: the cut was b12's Found, already filed as #1065 (b16, `wip`; second probe commented on the issue); the suite red at 18:07 was fixed by QA (8debcc6e) and the 18:25 suite was green, so no fix-forward. b15 goes first (Blocks: the GR move); its probe re-run on origin/develop 524c5286 reproduces the `HashMap` demotion, and the brief's probe line and residue note were refreshed (84fbfb99 committed project_members.rs). #1061 `wip` off (b12 pushed, closes on promote). Changelog owed for b12: "A struct field holding code that may pause, handed two views and giving one back, compiles: its result is the call's view (#1061)."
