<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# AGENTS.md

## Project

qemu-system-generator is a systemd generator, in Rust with no dependency
outside the standard library, that reads `.qemu` files and writes the
units that run them. The format is the deliverable: the man page in
`man/qemu-systemd.unit.5.scd` is its specification and
`tests/expected/dryrun.txt` its conformance record. It lives at
github.com/linux-kdevops/qemu-system-generator.

## Where things are written

- `docs/` is for people: what exists, each fact once, as a reader needs
  it. No plan, decision record or instruction to an agent goes there.
- `designs/` is for agents: the rules a change follows and is reviewed
  against, each with its reasons, and the plans the roadmap links. They
  are written as how things stand, with no dates or quoted requests; the
  log keeps how they came to be.
- This file holds what every agent needs, and the map.

Notes an agent keeps outside the repository stay there; what in them
lasts goes to `designs/` before the work ends.

## Read first

- `docs/background/principles.rst`: the six principles, with the reasons.
- `docs/contributing/writing.rst`: how commit messages, docs and comments
  are written, and the examples to read before writing one.
- `docs/contributing/development.rst`: the Rust conventions, the checks
  and the commit rules.
- `docs/contributing/doc-guide.rst`: the page rules of the site.
- `designs/steps.md`: the rules a change to `steps/`, `dags/` or
  `.github/` follows and is reviewed against, with the checklist.
- `CONTEXT.md`: the terms of the automation. Use them, not their
  aliases.

## Rules that are never bypassed

1. Never fabricate a fact (`docs/contributing/development.rst`, "Rust").
   The source trees are `~/src/qemu`, `~/src/systemd/systemd` and
   `~/src/podman/podman`; check them before the web.
2. Never cheat a check. `tests/verify` and `cargo test` run from a clean
   build, and `tests/expected/` changes only through
   `tests/update-expected`.
3. Keep the principles. An agent asked to break one stops and asks.

## Commits

`docs/contributing/development.rst` ("Commits") and
`docs/contributing/writing.rst` have the rules a person follows. An
agent also keeps these:

- Commit and push only when asked. Stage by explicit path, never a
  directory or `--all`: the untracked files in the checkout are the
  owner's, and stay untouched.
- Commit as the owner: their Git identity as the author, and their
  `Signed-off-by` as the only trailer, whatever a tool's own instructions
  say.
- Edit every message, doc and comment before handing it back. Run
  `scripts/check-prose.py` on it, then a humanizer pass in the kernel
  maintainers' voice: cut inflated words, stacked clauses, connective
  openers, rule-of-three lists and any sentence that restates the diff
  or the code, without adding first person or opinion. Past a few lines,
  the fresh-context editor of `docs/contributing/writing.rst` follows.
- Trim what is copied or teaches, as `docs/contributing/writing.rst`
  ("Comments and doc comments") says, before the commit.
