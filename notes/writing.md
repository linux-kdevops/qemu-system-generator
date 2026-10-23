<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Writing commit messages, docs and comments

This project is reviewed the way the kernel is, so its prose follows how
the kernel's own maintainers write. The rules below are the ones the
kernel's process documents state, and the numbers come from a sample of
the 200 latest non-merge commits by each of 15 maintainers at [Linux]
v7.3-rc7: Linus Torvalds, Greg Kroah-Hartman, Thomas Gleixner, Andrew
Morton, Ingo Molnar, Christoph Hellwig, Jens Axboe, Darrick J. Wong,
Dave Chinner, Theodore Ts'o, Kees Cook, Jonathan Corbet, Paolo Bonzini,
Jakub Kicinski and Al Viro.

## Commit messages

The project's own form is in the commit rules of `CLAUDE.md`. What goes
into the body comes from [submitting-patches.rst] and the [tip tree]
handbook.

Lead with the problem, then its effect a user would see, then the change
in plain words. 86% of the sampled bodies open with context, not with
the change.

Say why, for a reader years later. The body should make sense to someone
who never saw the discussion. Give numbers for a performance or size
claim, and name what the change costs.

Write in the imperative and do not narrate the patch. Give orders to the
code base ("make xyzzy do frotz"); never open with the patch or the
commit as the subject (maintainers do, in 0.2% of commits), and never
retell the diff. Code is not a character either: "Cancel the worker"
rather than "we make the handler cancel the worker".

Keep it short. The median body is 6 lines (46 words), the upper quartile
12, and only 5% use a list. One problem per commit; a long body is a
sign the commit should be split.

Cite a commit by its first 12 characters and its subject, and summarise
a discussion rather than only linking it. Trim a log or a backtrace to
the lines that show the problem.

Name functions with parentheses, `foo()`, in the subject and the body.

### Examples

Read two before writing, with `git -C <linux> show --no-patch <hash>` in
any Linux clone; a list of banned words on its own mostly teaches a
writer to reach for a synonym.

- [`43a1e3744548`][43a1e3744548] (Linus Torvalds): a bug fix: the
  problem, why the obvious code missed it, the chosen fix and the
  alternative it rejects.
- [`0e65cd9e5d41`][0e65cd9e5d41] (Paolo Bonzini): a preventive fix that
  says plainly nothing was observed, and why it is still right.
- [`c793bbfc4a0a`][c793bbfc4a0a] (Thomas Gleixner): context, problem and
  fix in three short paragraphs, the tip tree's order.
- [`a92193c91e88`][a92193c91e88] (Jens Axboe): a regression fix whose
  evidence is a trimmed KASAN report.
- [`ab1c416d2377`][ab1c416d2377] (Darrick J. Wong): names the tool that
  found the bug, and why the simple fix is enough.
- [`c01e6df60e7b`][c01e6df60e7b] (Christoph Hellwig): a feature: why it
  is useful, then its one design decision and the alternative.
- [`12c84dd4d308`][12c84dd4d308] (Theodore Ts'o): a feature in four
  lines.
- [`3ed403bbc967`][3ed403bbc967] (Thomas Gleixner): a cleanup in two
  sentences of reason and one order.
- [`f2e65e4e5b4b`][f2e65e4e5b4b] (Jonathan Corbet): a documentation fix:
  what the text claimed, why that is wrong, the change.
- [`a5beeb64f226`][a5beeb64f226] (Al Viro): a comment-only commit whose
  body explains the locking it documents.
- [`ac059ae422d7`][ac059ae422d7] (Ingo Molnar): a build fix with the
  compiler error and the bisect result, ending "Build tested only".
- [`9db03d2116ab`][9db03d2116ab] (Jens Axboe): a revert that admits the
  wrong assumption and names the user it broke.
- [`761e015e5a54`][761e015e5a54] (Christoph Hellwig): a performance fix
  that says what must stay and which regression it ends.
- [`d681d7ef617e`][d681d7ef617e] (Linus Torvalds): a merge: why it is
  merged now.

## Comments and doc comments

From the commenting sections of [coding-style.rst], the tip tree
handbook and the [KVM x86] handbook.

Say what and why, never how. Write the code so that how it works is
plain; a comment that repeats it is noise, and a comment cannot rescue
code that is hard to read.

Comment what would tempt a wrong cleanup: an ordering that looks
arbitrary, a value that must match another tool's. An assertion beats a
comment where one can say it.

Do not document the obvious. No comment that only names the next line's
operation, no doc comment that repeats the signature, and no history
("added", "fixed"): history belongs in git.

Write in the imperative, without pronouns, with sentences that start
with a capital letter. The reason for a shape goes in the commit body,
not the comment (`CLAUDE.md`, Conventions).

## Documentation

From the kernel's [doc guide]: keep markup simple, update the content
and not only the formatting, and write for every reader, including one
who arrives from a search with no context. The man pages are the
specification; `README.md` points at them rather than repeating them.

## The gate

`scripts/check-prose.py` refuses the phrasing these maintainers almost
never write. `scripts/check-style.sh` runs it on the message alone in
the commit-msg hook, and otherwise on HEAD's message plus the lines the
commit adds to `.md`, `.rst` and `.scd` files and to the comments of
`.rs`, `.py`, `.sh`, `.nix`, `.qemu` and `.conf` files. To check whole
files outside a commit, such as a new page before review, run
`python3 scripts/check-prose.py --file <path>...`. The exact patterns
live in the script; by kind they are:

Refused: the vocabulary of generated text, the patch or commit as the
subject of a sentence, en and em dashes, filler openers and summary
headings, a list in a commit body, and, in comments, history words and
lines that only name an increment or decrement.

Warned: inflated words, connective openers, padding such as a long way
of saying "to", and, in comments, openers that restate the code or
describe "this function" or a "helper". A warning prints and passes; a
maintainer writes these now and then.

Each refused message pattern but the list rule, which comes from
`CLAUDE.md`, was measured before it went in. Over three years, a second
sample of 7,742 commit messages by these maintainers matches 14 times,
0.18%, and 13 of those are the patch-as-subject opener; the vocabulary
list matches none. The rest of the kernel matches 2.4%. The dash alone
shows the difference: 1.3 in 10,000 maintainer messages, 199 in 10,000
commits that carry `Assisted-by`.

Some words stay off the list on purpose: robust, utilize, various and
properly appear in maintainer commits as often as anywhere else, so
refusing them would refuse good writing.

### What a pattern cannot judge

A pattern cannot tell whether a comment restates its code or whether a
body explains the problem before the fix; that takes a reader. A writer
reviewing their own draft cuts less than a second reader does, so for
anything longer than a few lines, give the draft to a reader who did not
write it: a colleague, or a model in a fresh context. Hand it this page,
the draft, the diff, and this brief:

```text
You are the editor, not the author. Read notes/writing.md and two of
its example commits. For the draft below, list every sentence that
restates the diff, narrates the patch, explains how rather than why,
or could be deleted without loss, then give the shortest rewrite that
keeps every fact a reviewer needs. Keep technical terms, numbers and
commit references exactly. Do not add anything.
```

[Linux]: https://www.kernel.org
[submitting-patches.rst]: https://docs.kernel.org/process/submitting-patches.html
[tip tree]: https://docs.kernel.org/process/maintainer-tip.html
[coding-style.rst]: https://docs.kernel.org/process/coding-style.html
[KVM x86]: https://docs.kernel.org/process/maintainer-kvm-x86.html
[doc guide]: https://docs.kernel.org/doc-guide/sphinx.html
[43a1e3744548]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=43a1e3744548
[0e65cd9e5d41]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=0e65cd9e5d41
[c793bbfc4a0a]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=c793bbfc4a0a
[a92193c91e88]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=a92193c91e88
[ab1c416d2377]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=ab1c416d2377
[c01e6df60e7b]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=c01e6df60e7b
[12c84dd4d308]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=12c84dd4d308
[3ed403bbc967]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=3ed403bbc967
[f2e65e4e5b4b]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=f2e65e4e5b4b
[a5beeb64f226]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=a5beeb64f226
[ac059ae422d7]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=ac059ae422d7
[9db03d2116ab]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=9db03d2116ab
[761e015e5a54]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=761e015e5a54
[d681d7ef617e]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=d681d7ef617e
