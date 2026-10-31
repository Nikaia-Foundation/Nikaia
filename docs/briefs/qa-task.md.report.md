Result: done
Cause: origin/develop (not the unpushed briefs commit 60771737): pre-push, self-hosted.svg stale after the share moved
Cause: origin/develop, a99ce32 (#1049) vs the test: test a_type_argument_is_written_with_brackets_and_emitted_with_angles, a sequence argument of a built-in is now emitted as a group `text((alpha1 digit*))`
Fixed: badge redrawn with scripts/badges.py; the test's fixture rule now takes `text(alpha1)`, so it still checks that a built-in without type arguments is unchanged (assertion not loosened, #1049's output is intended and covered by grammar_a_builtin_takes_a_sequence.nika)
Tests: that grammar_lowering test ok (profile fast); badges.py --check ok; fmt ok; pre-push-check.sh ok
Found: none. Note: the debug profile fails with a winnow recursion limit; the fast profile (the gate's) is fine. The modified rs2nika nikaia.{lock,proofs,contracts,derived} files were in the tree before I began and are left alone.
Files: assets/badges/self-hosted.svg crates/nikaia/tests/it/grammar_lowering.rs
Commit: ef8d8320 (badge), 84aa402a (test)
