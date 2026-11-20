# ADR-decision-records-v2: A Topic Has One Record, Named `ADR-<topic>-v<n>`; a Change in Substance Makes the Next Version, and the Previous One Goes to the History

**Status:** Accepted
**Date:** November 19, 2026 (owner)
**Answers:** how a decision record is named and changed, so that a topic is read in one document, a citation in an old commit message still finds the text it meant, and no two records on one topic are live side by side
**Supersedes / Related:** the record it replaces, now in [`docs/history/adr/`](../../history/adr/README.md) (a number per topic counted records side by side, and a change was made in place); [`README.md`](README.md) (*Writing a new one*, *Reserved numbers*), [`TEMPLATE.md`](TEMPLATE.md), `scripts/check-adr-refs.nika`, [#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904) (the consolidation of the earlier records)

## 1. Question

The scheme before this one named a record `ADR-<topic>-<n>`, the number counting
the records of a topic. That let records on one topic live side by side:
`grammar-1` and `grammar-2`, `views-1` to `views-3`, `collections-1` and
`collections-2`, each deciding a different part, and next to them the numbered
records on the same subject (a numbered grammar record and `grammar-1` both decided that an action
stands after `->`). A reader looking for the rules of a topic reads several
documents and cannot tell which one holds.

A change was made in place, with a *Changed by* note. A citation in a commit
message (`after ADR-grammar-syntax-v1 D13`) then points at a text that no longer says what
it said when the commit was written, and commit messages are not rewritten.

How is a record named and changed so that one topic is one document, and an old
citation still finds what it meant?

## 2. How others do it

| | one subject is | a change is |
|---|---|---|
| Nygard's ADRs, MADR | many records, one per decision | a new record that marks the old one *superseded* |
| IETF RFCs | one document per version | a new RFC that *obsoletes* the old one; the old stays readable |
| W3C, WHATWG | one living document | edited in place; dated snapshots for citation |
| Python PEPs | one document per proposal | edited until final, then a new PEP |

The IETF form fits both goals: one document holds a subject, and a version
cited once keeps saying what it said.

## 3. Decisions

### D1 — A record is named `ADR-<topic>-v<n>`.

`<topic>` is one to three words, each lowercase letters and digits starting
with a letter, joined by `-`; the general word comes first and the narrower
after it: `grammar-syntax`, `grammar-build-time`, `database`. No word is a `v`
followed by digits. `<n>` is the version of the topic's record, from 1. The file
is `adr-<topic>-v<n>.md`, its first line `# ADR-<topic>-v<n>: <the decision>`, and
a decision in it is cited by the name and its number, as `grammar-syntax-v1` D4.

**Why.** The `v` says that the number is a version of one document, not one of
several records on the topic. **Rejected:** no number (an old citation would
point at whatever the record says today); a number without `v` (it reads as
"the second record on grammar", which is what led to records side by side).

### D2 — A topic has exactly one live record.

A topic is cut so narrow that one document holds it and a reader reads it whole:
not `grammar`, but `grammar-syntax`, `grammar-input`, `grammar-parallel`. A
record that touches several topics takes the one it decides about and names the
others under **Supersedes / Related**. A new topic is taken when none of the
existing ones fits; the index lists the topics in use.

**Why.** One live record per topic is what makes "read the record" an answer.
`check-adr-refs.nika` refuses two live versions of one topic.

### D3 — A change in substance makes the next version; the previous one goes to the history.

When a decision of a topic is added, changed or withdrawn, the topic's next
version states all its decisions in their current form, the previous version
moves to [`docs/history/adr/`](../../history/adr/) with a row in its README, and
every live citation moves to the new version in the same change. Nothing live
cites an archived record.

A decision that stays keeps its number in the new version; a new decision takes
the next free number; the number of a withdrawn one is not given again. So a
citation moves by its name alone, and D4 of `grammar-syntax-v1` and D4
of `grammar-syntax-v2` are the same decision where it did not change.

Made in place, without a new version: wording, a typo, a dead link, the index's
**Built** column.

**Why.** A version that was cited keeps saying what it said, in the history,
so a commit message that cites it is still followed. **Rejected:** *Changed by*
notes in a record of this scheme (they make the reader assemble the current rule
from several places; the new version states it once).

### D4 — The records of the earlier schemes are consolidated when they are changed.

A numbered record (`ADR-338`) and a record of the scheme before (`ADR-views-v4`)
keep their names while nothing changes them. A change to a decision in one of
them is made by consolidating: the topic's next `-v` record takes the decisions
of every earlier record on that topic, and those records move to the history
in the same change. A record of the scheme before continues its count: after
`regex-1` comes `regex-v2`. The remaining consolidation is listed in #904.

### D5 — A name is claimed before it is written.

The index's **Reserved numbers** table holds names of every scheme: a row
`regex-v2` claims that name as a row `385` claimed that number. A row is
deleted in the commit that adds the record.

### D6 — `check-adr-refs.nika` checks the schemes side by side, and moved to Nikaia with the rename tool.

It checks that every record has a valid name, one index row and a title that
agrees; that a topic has at most one live `-v` record; that every `Dn` and `§`
cited resolves; that no live file cites an archived record; and that every topic
name cited names a record. It was kept in Python until `std::regex` existed, and
moved to Nikaia (`scripts/check-adr-refs.nika`, #2263) after the rename tool of
#904, which uses the same patterns.

## 4. Consequences

* A record is read whole and is the only one on its topic.
* Every change in substance moves the citations of one topic. The rename tool
  of #904 does that; a decision that keeps its number moves by name alone.
* The history grows by one file per change, which is what keeps old citations
  followable.
