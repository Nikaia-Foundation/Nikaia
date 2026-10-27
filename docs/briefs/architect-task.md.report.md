Result: done
Batch: b-drive-restart.md, b887-tuple-nullable-wrap.md, e12-3a.md
Owner: none
Note: the cut at b885 was false (d942502 is on develop; #885 done, version raise owed from its Changelog line). The running driver predates 5a91e92's rewrite; restart drive.nika so it takes effect. b-drive-restart makes the driver stop with `RESTART` (exit 3) when its own source changes.
