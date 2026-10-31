Result: done
Cause: 316e0721 (tiers; the lowering now writes TrustedMap/TrustedSet) left lowered/project_members.rs stale with HashMap: step lower-std "lowered drift"
Fixed: committed the file as `nikaia lower-std` writes it (7 lines, HashMap/HashSet to TrustedMap/TrustedSet)
Tests: nikaia lower-std rerun: second run changes nothing (fixpoint); cargo build --profile fast -p nikaia ok with the file; pre-push-check ok
Found: none
Files: crates/nikaia-std/src/tools/lowered/project_members.rs docs/briefs/qa-task.md.report.md
Commit: lowered: project_members.rs as lower-std writes it (TrustedMap, TrustedSet)
