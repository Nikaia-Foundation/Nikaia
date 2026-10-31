Result: done
Cause: 91d5eb6b (ADR-384 record added without its index row), step pre-push (check-adr-refs.py: "ADR-384: a record with no index row")
Fixed: added the ADR-384 row to docs/specification/adr/README.md
Tests: scripts/pre-push-check.sh exit 0 (fmt, check-adr-refs, badges, bump check)
Found: none
Files: docs/specification/adr/README.md
Commit: adr: index row for ADR-384 (pre-push's check-adr-refs was red)
Changelog: none
Pushed: local 003eeea6fa830c171404a92376d0a19a58560f8a
