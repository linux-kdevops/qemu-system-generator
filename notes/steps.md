<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Steps, DAGs and CI

`steps/` reproduces what `docs/bootstrap/` documents, `dags/` wires the
steps for [Dagu], and `.github/` runs those DAGs on GitHub-hosted runners.
The conventions are borrowed; each section names where it comes from, so a
question goes back to the source rather than to this page.

## Principles

From kdevops-ng's five rules
(`~/src/kdevops-ng/docs/concepts/principles.rst`), applied here:

- Nix first. A step runs in the flake's `steps` shell under
  `nix develop .#steps --ignore-env`, never a tool that happens to be on
  the host's `PATH`.
- systemd first. A machine is a `qemu-system@` unit and a step drives it
  with `systemctl --user`; no daemon, PID file or wrapper of our own.
- User mode only. No step runs `sudo`. The one-time root setup is written
  in `docs/bootstrap/host.rst` for a person to run, and
  `steps/github/prepare_runner` runs the same setup on a throwaway GitHub
  runner only, called from no DAG a developer runs
  (`~/src/kdevops-ng/docs/getting-started/requirements.rst` has the
  original list).
- Write only where the guides write: `~/bootstrap`,
  `~/.config/qemu/systemd`, the generator's links in
  `~/.config/systemd/user`, `~/.local/state/qemu-system` and the user's
  Nix profile. `~/.ssh/config` stays the user's.
- Commands a developer can run. Every step prints what it runs
  (`set -o xtrace`) and its usage ends with the command line that runs it
  by hand.

## The step interface

From `~/src/kdevops-ng-minimal-dagu/CLAUDE.md` and its README's "Steps":
a step is a standalone executable, `steps/<subsystem>/<verb>`; every input
is a long option; nothing comes from the environment but `HOME` and
`XDG_RUNTIME_DIR`; an unknown option prints the `## Usage` header; results
are `name=value` lines appended to `--output`, which is
`$DAGU_OUTPUT_FILE` in a run and `/dev/stdout` by hand.

## Names

From `~/src/kdevops-ng/CLAUDE.md` ("Subsystem dirs, verb steps, thin
flows", "Canonical upstream vocabulary") and
`~/src/kdevops-ng/docs/concepts/flows.rst`, with Windmill's flows read as
DAGs: a subsystem directory groups one concern (`host`, `ssh`,
`generator`, `guide`, `machine`, `github`); a step is named for its action
in the imperative, `verb_object` in snake_case when it takes an object
(`generate_key`, `prepare_runner`); a DAG is `dags/<verb>.yaml`; a knob
keeps the upstream tool's own name.

## The guides are the source

`steps/guide/run` turns a guide into the script that leaves the host as
the guide does, so no step restates a file or a command a guide already
shows. Only what drives a machine (start, the SSH checks, stop) and what
needs root stay out of the replay. `docs/bootstrap/host.rst`, which every
guide starts from, is replayed first, as a step of its own. A guide that a
run cannot follow is a guide to fix.

## Thin DAGs, dumb CI

From `~/src/kdevops-ng-minimal-dagu/CLAUDE.md` ("CI is dumb"): a DAG only
wires steps, and a workflow under `.github/workflows/` only prepares the
runner, runs the same `dagu start` and `dagu status` a developer runs, and
collects what the DAG wrote. Anything that computes, parses or decides is
a step. Dagu is that project's patched build (`nix/dagu.nix` and
`nix/dagu-patches/` there) until the patches land upstream.

## Runners

GitHub's [runner reference] and [limits], as of October 2026: a public
repository's standard Linux runner has 4 CPUs, 16 GB of RAM and a 14 GB
SSD, free and unmetered; a private one has 2 CPUs and 8 GB and spends the
account's minutes. A job runs for at most 6 hours. The Free plan allows
20 concurrent jobs per account, so a repository under an organization
shares them with the organization's other repositories.

[Dagu]: https://github.com/dagucloud/dagu
[runner reference]: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
[limits]: https://docs.github.com/en/actions/reference/limits
