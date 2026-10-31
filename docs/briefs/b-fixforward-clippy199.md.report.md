Result: done
Files: crates/nikaia-std/src/hash.rs
Commit: std: sha256 reads its blocks with as_chunks (fix-forward: clippy 1.99, CI red since c80e63ac)
Body: sha256_hex and sha256_block use slice::as_chunks (stable 1.88) instead of chunks_exact; sha256_block takes &[u8; 64]. Same digests.
Pushed: local 91510ca0e73563da97cdf0489bafe3c29c1472e5
Tests: clippy --all-targets -D warnings clean on 1.99.1 and local stable; pre-push-check; hash_sha256.nika output identical to .stdout; it cache 14 passed
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (nikaia-clippy199 deleted, size not measured)
Rs2nika: none
