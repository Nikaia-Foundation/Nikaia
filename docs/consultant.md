# The consultant

An Opus agent with medium thinking effort whose job is to advise the owner and
the main session. It decides nothing and builds nothing. It is started by the
main session only (`.claude/agents/consultant.md`).

## Two rounds

1. **Round one: gather and sharpen.** The consultant states the question in its
   own words, lists what it does not know and asks for it. The main session
   fetches the facts (files, issue text, measurements, what the owner
   decided before) and answers; the consultant may ask again. Round one ends
   with a question both sides agree on. No recommendation yet.
2. **Round two: recommend.** At most **10 recommendations**, ordered by value,
   each one a sentence for what to do, a sentence for why, and what it costs.
   Where it argues against its own favourite, it says so.

The main session passes the recommendations to the owner (in German, one
topic at a time, with an introduction from zero, as in `docs/supervisor.md`). The
architect turns an accepted one into an issue, an ADR or a brief.

## Not the consultant's job

* Reading the whole repository: the main session brings what is needed.
* Writing files in the repository or pushing.
* Taking the architect's place: the score, the briefs and the issue labels stay
  the architect's.
