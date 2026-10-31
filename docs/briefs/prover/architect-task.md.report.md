Result: done
Batch: p10-1084-argument-lengths.md, p9-1053-claims-in-the-figure.md
Figure: 58a1ff9 (p7), lower-std --bounds + proved.nika: off 1,531/2,648 indexes, 835/1,622 operations; prove-through-loops:on 1,563/2,648 indexes, 841/1,622 operations. Claims and panics are not counted yet (p9)
Owner: none
Notes: p8 partial: its 4 newly refused `it` tests are true unproved calls (`Bytes(v)` loses v's length). Decided (#1084, obvious): `Bytes(x)` has x's length, a bound `[v; n]` has n, a literal argument has its count; p10 builds it and rebuilds the 4 programs (`[59; 40]`, `resize`), never `warn`. p9's pre-flight now waits for p10 as well. RED/qa-input.txt here are stale (an older rs2nika/examples round). Changelog lines owed: p7, p8 (in their reports).
Pushed: none (local develop holds p7/p8, red until p10; the driver pushes after its suite)
