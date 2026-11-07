<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Steps, DAGs and CI: rules for agents

An agent that writes, changes or reviews a step under `steps/`, a DAG
under `dags/` or a workflow under `.github/` follows this file, and a
reviewer holds the change to it. S1 to S10 make the last principle of
`docs/background/principles.rst`, "Simple steps", checkable; `CONTEXT.md`
has the terms.

## 1. Who owns what

- Nix owns every tool, pinned, and the builds a run repeats. A DAG builds
  the kernel and QEMU a guide builds as derivations (`steps/knix/flake.nix`
  through knix's `lib.buildKernel` and `lib.buildQemu`), so equal inputs
  build once and a later run reuses the store path; the guide keeps
  teaching `make` in a `nix develop` shell, which registers nothing it
  builds. The two then match in function, not in paths.
- systemd owns whatever runs for a while: the machines, the generator's
  units, and waiting on a unit.
- Dagu owns control flow: order, retries (`retry_policy`), what runs as a
  run ends (`handler_on`), and which machines a run deploys.
- A step owns the commands a person types to reach one goal on one
  object, and nothing a DAG can decide for it.

A guide and its steps are independent documents, kept by hand: neither
reads, includes or names the other. The steps prove that what a guide
teaches works, so a change to a guide needs the matching change to its
steps, in the same series. They may also do what no guide explains, for
CI's sake; a DAG's `description` says what its run covers. Steps are not
generated from a guide: a parser decides by pattern what to run and what
to drop, so a new command in a guide would run or vanish silently, and a
log of a generated script is no record of commands a person typed.

## 2. The rules

A review finding cites a rule by its number.

### S1. One goal, one object

A step does one thing to one object, named by a long option (`--name`),
and a step with two goals becomes two steps. A step whose object is fixed,
such as a guide's flake, takes none. Parts of one object, such as the
disks of one machine or the device nodes of one host, are still one
object.

### S2. No loop over work items, and no retry, sleep or wait

A work item is anything a DAG node could run, fail and rerun alone: a
machine to deploy, an image to fetch. A list of them is a node each; a
retry is the DAG's `retry_policy`; waiting is a step that fails until its
goal is reached, repeated by the DAG.

### S3. A step never runs another step

A step sources only `steps/base` and a `--base` file, and never calls
`nix develop` (section 3, "Environment").

### S4. The tool built for the job, before Bash of your own

systemd, Nix, QEMU, Git and the guest's own tools come first. Bash that
copies a tool's logic breaks this rule.

### S5. The DAG decides

A step does not branch on an earlier step's result to choose how it
works. A table that maps a machine's name to its values, such as its vsock
context id, is data, not a choice.

### S6. A transcript

A step prints every command it runs (`set -o xtrace`) and names every
file it writes, so its log is a recipe a person can follow.

### S7. One home for each piece of logic

What more than one step needs lives once: the places in `steps/base`, a
file in a `files/` directory with one step that writes it.

### S8. No code for a case that has not happened

Keep a fallback only when it is known to fire, and drop one that works
against the design.

### S9. Readable

Long options everywhere, and comments as `docs/contributing/writing.rst`
says.

### S10. The principles

A change keeps the principles of `docs/background/principles.rst`: no
`sudo` on the host but in `steps/github/prepare_runner`, which does a
GitHub runner's one-time root setup and refuses to run outside a GitHub
Actions job; no tool from outside the flake.

## 3. Writing a step and a DAG

### Names

A step is a Bash executable, `steps/<subsystem>/<verb>`. A subsystem is
a guide (`disk`, `uki`, `direct`, `knix`, `packages`, `cloud`), a concern
the guides share (`host`, `ssh`, `generator`, `machine`, `github`), or a
DAG of CI's own (`nested`, `nvme`, `runner`, `virtfs`). The verb is in
the imperative, `verb_object` in snake_case when it takes an object
(`generate_key`).

### The header

After the shebang and the SPDX line, a step has a `## Usage` block: its
one form, one sentence of what it does, and the command line that runs it
by hand. `--help` prints it, and any other argument prints it to the
standard error and exits with status 2. It takes the happy path, with as
few options as possible.

### Files

A file a step writes lives as a file of its own in
`steps/<subsystem>/files/`, so it can be read and checked without running
the step. The step copies it, or fills in its `$name` placeholders with
`envsubst`, given the list of names to fill, so that Nix's `${...}` and
systemd's `%` specifiers pass through as written. A file copied from a
template keeps only what the step uses (`docs/contributing/writing.rst`).

### The base

`steps/base` holds the places, each defaulting from the one above it:
`WORKBENCH_DIR` (`~/bootstrap`) gives `WORKTREES_DIR` and `SYSTEM_DIR`
(`~/bootstrap/system`), `SYSTEM_DIR` gives `SSH_DIR`, and `SSH_DIR`
gives `SSH_KEY`. A step that works in a place takes `--base FILE`, a
user's own base that sets only what it changes and is read first.
Nothing else comes from the environment but `HOME` and
`XDG_RUNTIME_DIR`. A step writes every file that names a place itself,
so the place is in the file as written.

### Environment

The caller enters the flake's default shell once and runs a step or
`dagu start` in it:

```shell
nix develop --ignore-env --keep-env-var HOME --keep-env-var XDG_RUNTIME_DIR
```

No step or DAG calls `nix develop`, and none uses a tool that happens to
be on the host's `PATH`. A step that only strings commands together is
Bash; a program that does what no handful of commands can may be Python,
as `scripts/check-prose.py` is, but stays out of `steps/`.

### DAGs

A DAG node's `id` is the step it runs, `<subsystem>_<verb>`
(`host_check`, `ssh_generate_key`), followed by the value of the
argument that tells two runs of that step in one DAG apart
(`cloud_fetch_debian`), or, when the arguments are the same, by what
sets the later run apart (`machine_wait_reboot`).

A DAG (`dags/<guide>.yaml`) is `type: chain`, takes no parameters, and
holds only the control flow: it retries `machine/wait` until the guest
answers, stops the machines from `handler_on: exit`, whether the run
passed or failed, and ends by removing every machine it deployed, so a
run that passed leaves no `.qemu` file or state directory behind and a
failed one keeps them to look at. Each guide's machines have names,
vsock context ids, forwarded ports and a LAN group of their own, so runs
of different DAGs may overlap; Dagu's default `max_active_runs` of 1
keeps two runs of one DAG apart.

### CI

A workflow under `.github/workflows/` prepares the runner, then runs the
`dagu start` and `dagu status` a person runs, and keeps Dagu's logs. Its
matrix holds every DAG but `runner`, which runs the workflow itself, and
every push runs them all. Dagu is built by `nix/dagu.nix` until its
patches land upstream.

GitHub's [runner reference] and [limits], as of October 2026: a public
repository's standard Linux runner has 4 CPUs, 16 GB of RAM and a 14 GB
SSD, free and unmetered; a private one has 2 CPUs and 8 GB and spends the
account's minutes. A job runs for at most 6 hours. The Free plan allows
20 concurrent jobs per account, so a repository under an organization
shares them with the organization's other repositories.

## 4. Kept on purpose

A reviewer accepts these. Any other exception needs the owner's consent
and a line here.

- `steps/machine/wait` asks the guest over SSH whether its systemd has
  finished starting: QEMU sends no readiness of its own, and
  `designs/guest-readiness.md` has the path that would replace the SSH
  question.
- `steps/generator/install` looks for the profile in `~/.nix-profile` and
  in `~/.local/state/nix/profile`, since Nix links one or the other
  depending on `use-xdg-base-directories`.
- `steps/host/check`, `steps/nvme/build`, `steps/nvme/check` and
  `steps/direct/install` loop over the parts of one object: the device
  nodes of the host, the disks of one machine, the drop-ins of one
  machine.

## 5. Not yet conforming

Known departures, each waiting on its own change:

- `steps/cloud/fetch`, `steps/nested/fetch` and `steps/runner/fetch`
  download images with `curl` while the step runs, where Nix would pin
  them as fixed-output derivations (S4, S10).
- `steps/runner/provision` runs a list of the runner image's installers in
  one loop (S2).

## 6. How a change is made

1. Ask the owner before a change to what a person sees, a fallback or an
   exception in section 4.
2. Run the DAG of every guide whose steps changed, after asking: a run
   replaces `~/bootstrap/<guide>` and the machines' `.qemu` files. Put its
   time and result in the commit body.

## 7. Review checklist

For each changed step:

- S1: one object and one goal, or would its usage line need an "and"?
- S2: does it loop over work items, retry, sleep or poll?
- S3: does it run another step, or call `nix develop`?
- S4: does a tool already do what its Bash does?
- S5: does it choose how to work from an earlier result?
- S6: does the log read as commands, and name every file written?
- S7: is a function, value or file also in another step?
- S8: does it handle a case nothing shows happening?
- S9: long options, and comments that say why?
- S10: `sudo`, or a tool from outside the flake?
- Section 3: its name, header, files and DAG node as written there?

For each changed DAG: it holds only control flow, as section 3, "DAGs",
says. A finding names the rule, the file and line, what fails, and the
smaller design. "Justified" stands only with a line in section 4.

[runner reference]: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
[limits]: https://docs.github.com/en/actions/reference/limits
