# How we build Nikaia

This page says how the work is organised, and what is measured and what is not yet.
It is a description of a way of working that is still changing.

## The owner and the standard

The owner sets the goals, the stages and the priorities, and decides what is
his to decide. The standard is high on purpose: a defect met on the way is
fixed in the compiler, with a test, as its own commit. It is not worked
around in our own code, because a workaround is technical debt. Nothing goes
to `main` unless it passed CI; everyone else pushes to `develop`, in small
commits, often.

## Several agents, each with one job

The owner talks to one session, the main session. It takes his ideas, keeps
them until they are needed, and hands them on. The work itself is split:

* **The architect** is an Opus agent with few tools. It reads, searches,
  edits documents and works on GitHub issues. It decides what is built next
  and how, writes the decision records and the specification text, and
  returns one work order. It cannot start other agents.
* **The builders** have six file and shell tools and no GitHub. A builder takes
  one or two steps from a work order, commits, pushes, reports and ends.
* **The main session** starts the builders from the architect's orders, carries
  reports back to the architect, and watches the cost.

Until now builders have run one after another. A second one, working at the
same time on parts that do not depend on each other, comes next. Whether the
compiler can be cut so that this works without conflicts is a question we
are examining now (`docs/modular-compiler.md`).

## Tokens are the cost, so we count them

An agent's cost is the context it re-reads on every turn, not what it writes.
In a measured long run the context grew to 566-626 k tokens, and 71-78 % of the
cost came after it passed 300 k. So builders are short: about 120 turns, or
about 200 k tokens of context, then a fresh agent starts from the repository,
not from a long prompt. The same arithmetic says the same work with an agent
restarted every 120 turns would cost about one third; that figure is computed,
not yet confirmed.

We measure in counts, never in time: tokens per Rust line moved out of the
compiler (`docs/tokenspeed.md`), and instructions from callgrind for
performance. The same binary runs 1.4-1.9 times differently from one day to
the next on our machines, so a time proves nothing. Whether the new way of
working is faster is being measured now; we do not claim it yet.

## The language builds itself

The goal is that Nikaia hosts itself. Today 68.8 % of the toolchain is
written in Nikaia (`scripts/self_hosting.py`; the figure moves with every
step). A converter, `crates/rs2nika`, translates a Rust function into a first
draft in Nikaia, marks with TODO where it cannot, and names the functions it
still needs. The builder starts from that draft, not from an empty file.

Every helper tool is written in Nikaia, not in Python or shell. That is a
principle, not a convenience: whatever blocks or chafes on the way, a missing
library entry, a gap in the syntax, a defect in the lowering, is a finding,
and the finding is fixed in the compiler.

Where we stand is on the [changelog](CHANGELOG.html); the reasons for the
project are in the [manifesto](manifesto.html).
