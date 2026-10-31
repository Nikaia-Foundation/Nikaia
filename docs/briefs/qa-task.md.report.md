Result: done
Cause: 91d5eb6b (already pushed-range brief commit) added docs/specification/adr/adr-384.md without its index row; pre-push step (check-adr-refs.py) red
Fixed: added the ADR-384 row to docs/specification/adr/README.md
Tests: scripts/pre-push-check.sh green (fmt, ADR refs, badges, bump)
Found: none (the "Unused import ProbePlain" warning in the suite log is a lint note in scripts/gate.nika, not a failure)
Files: docs/specification/adr/README.md
Commit: ADR index: add the row for ADR-384 (Entry::or_default, Seq::cloned)
Changelog: none
Pushed: local eadeb8f8b1342d97ed78392bf27a6de6382e30bf
