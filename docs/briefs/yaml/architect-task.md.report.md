Result: done
Batch: y7b-1009-let-reads.md, y4c-1003-finish.md
Owner: none
Pushed: a8ff9d31
Notes: y7a is done (a7f3843f, Fixes #639; its Changelog line in its report, version raise owed). Its `Gate: red` was develop's `sequences` guard (18/16 since 75e25ed8). The guard went red with this lane's 43312d30, but the main lane's `b-fixforward-corpus-unanswered.md` already owns the fix (first in `docs/briefs/batch.txt`), so this lane does not duplicate it. CI is red on that test only.
Probes (develop 1e179d9c): json's `hex4` proves its range once it carries `assert(n <= 0xFFFF)` (ADR-359 D3). The caller's `scalar::from(cp)` is refused for four prover defects, narrowed into y7b and commented on #1009.
Process: builder.md's gate and builder-facts' new *Red on develop now* section mean a guard already red on develop no longer cuts a batch. architect.md: a prover brief probes at the limit it claims (from y7a's debrief).
Issues: #928 *To build* updated (ready + wip); #1009 commented (ready).
