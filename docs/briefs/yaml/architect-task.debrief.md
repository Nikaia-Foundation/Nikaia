1. **Most cost:** reproducing the defect. The builder's probe passed on today's binary, so I wrote about 20 variants before I found the trigger: the struct's field has to be a view, built from a `trim()` inside a function rather than from a literal in `main`. Before that I cut down the real draft to see which arm was refused. Separately, my first commit carried only the two renames: one bad path made `git add` stage nothing new, and I chained the commit after it with `;`.
2. **Looked up:**
   - how `nikaia test` handles an output test (it needs an existing `.stdout`, even an empty one, before `--bless` writes it);
   - where the hand-back logic lives (`views.nika`, `handed_back`, `rsrc_*`);
   - #928's body, to edit *To build*;
   - which brief ends with the rules block (y27).

   The brief could have given the first two. The lane facts could say that `y27-load.md` currently holds the rules block.
3. **Checks:**
   - Useful: cutting the draft's `walk` arm by arm; the standalone repro (w1–w4); one scratch package run of the whole draft with `walk` rewritten. That run showed #1085 is the only defect left.
   - Wasted: the first scratch package run, with no `.stdout`, so nothing was blessed. The `pub struct` probes were a detour into an unrelated tier effect. About 10 probes with literal-built structs, all passing.
4. **One change:** builder.md should require every `Found:` probe to be run by the builder in a scratch file, not in the draft's context, with its output pasted, and to fail on today's binary. y21's probe would have failed that check, and I would have saved most of my turns. Second: never chain `git add … ; git commit`; use `&&`.
