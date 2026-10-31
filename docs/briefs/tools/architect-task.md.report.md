Result: done
Batch: j4-1019-members.md j5-1019-comptime-field.md
Owner: none
Pushed: rejected (local c3171f6; behind origin/develop, the driver rebases)
Decided: a `T?` crosses from build time where `T` does (`null` a const; `String?` arrives as `ref String?`), architect, obvious; Part II 10.2 sentence added in c3171f6 (#1087), so the supervisor owes a version raise with j5's build.
Filed: #1086 (a fieldless attribute struct is NK1135, `ready`, for the bug lane, no wip); #1087 (a comptime binding reads a reflected field and calls a function over it; T? crosses; `ready` + `wip`, brief j5). Commented #906 with the probe that `comptime` value parameters do not parse (j3's Deviation).
j3: done (7fe9f88, Part #1019); its Found (check-adr-refs on ADR-384) was another lane's, already fixed on develop (396ca42). #1019 To build: 3a ticked, 3b' (#1087) added before 3c.
Next after this batch: j6 = 3c (`name:` per field while building, NK1311, NK1312), on j4 and j5.
