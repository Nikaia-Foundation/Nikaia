Result: done
Files: crates/nikaia-std/src/fs.rs crates/nikaia-std/src/time.rs crates/nikaia-std/std.contracts docs/specification/30-nikaia-tooling.md docs/specification/adr/adr-319.md docs/specification/adr/adr-368.md tests/compiler/tests/fs_metadata_times.nika tests/compiler/tests/fs_metadata_times.stdout
Commit: std: fs::Metadata::modified, links, file_id and time::since_epoch
Body: fs::Metadata gains three fields filled in fs::metadata (modified as the Duration since the Unix epoch, zero where unknown or before it; links from MetadataExt::nlink, 1 off Unix; file_id as (dev, ino), None off Unix) and three sync readers with touches = []; time::since_epoch reads SystemTime against UNIX_EPOCH, zero before it, with no touches as time::now. Part III 17.1 names the readers and their off-Unix answers, the std::time bullet names the wall clock; ADR-319 section 7 and ADR-368 section 6 are the amendments (#922).
Part: #922
Tests: fs_metadata_times new, fails on the unfixed compiler (NK1171 for since_epoch, modified, links, file_id) and passes; nikaia test --both-settings --project tests/compiler 473 passed 0 failed; cargo test -p nikaia-std --lib fs:: time:: 25 passed; it filters fs metadata time 161 passed; clippy -p nikaia-std 1.97 and 1.99.1 clean; lower-std left lowered/ unchanged; pre-push-check green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: std: `fs::Metadata::modified()`, `links()` and `file_id()`, and `time::since_epoch()`, the wall clock to compare a file's time with (#922, ADR-319, ADR-368)
Owner: none
Rs2nika: none
Disk freed: 0 GB (clippy 1.99 target removed; cache 4.2G, 5.8G free, new entry run-bundle/3f8b93b88f7f213d 425 MB)
Pushed: local a9094a38c56f8e9c037c2951a059c789b1b38b2a
