Result: done
Batch: m2-modules-tests.md s3-fs-canonical.md
Owner: none
Commit: be20f0d (architect: batch m2, s3-fs-canonical; move briefs check std counterparts)
Cut: p3a on a Deviation, a real one but the brief's fault (it moved `shown_path`, whose `canonicalize`/`strip_prefix` std lacks). P3a done otherwise (b8547a6, 80.3 %, Changelog none). Rule added to docs/architect.md, *Briefs and reports*.
Decided: ADR-319 D3 amendment, `Path.strip_prefix -> Path?` and `fs::canonical(path, root) -> Path throws` (text in s3's brief, built with the code; a std change, so the supervisor owes a version raise once s3 reports its Changelog line).
CI: green to 025f3d5, 60ce1f9 running. b673-regex-dynamic.md (cd86c2a) belongs to another lane and is not in this batch.
Next: P3b, P4 (both need S3), K1 after the converter's *collect into an Option*.
