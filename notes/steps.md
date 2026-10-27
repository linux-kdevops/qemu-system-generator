<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Steps, DAGs and CI

`steps/` holds the commands that do what each guide of `docs/bootstrap/`
teaches, `dags/` wires them into runs for [Dagu], one DAG per guide, and
`.github/` starts those DAGs on GitHub-hosted runners. `CONTEXT.md` has
the terms.

## Steps

A step is what a person would type: a bash executable,
`steps/<subsystem>/<verb>`, that prints every command it runs
(`set -o xtrace`) and whose `## Usage` header ends with the command line
that runs it by hand. It takes the happy path, with as few options as
possible, and decides nothing a DAG can decide for it: no retry loop, no
waiting, no branching on an earlier step's result.

A subsystem is a guide (`disk`, `uki`, `direct`, `knix`, `cloud`) or a
concern the guides share (`host`, `ssh`, `generator`, `machine`,
`github`). A step is named for its action in the imperative,
`verb_object` in snake_case when it takes an object (`generate_key`).

A file a step writes lives as a file of its own in
`steps/<subsystem>/files/`, so it can be read and checked without running
the step. The step copies it, or fills in its `$name` placeholders with
`envsubst`, given the list of names to fill so that Nix's `${...}` and
systemd's `%` specifiers pass through as written.

A guide and its steps are independent documents, kept by hand: neither
reads, includes or names the other, and a change to one needs the
matching change to the other.

## The base

`steps/base` holds the places, each defaulting from the one above it:
`WORKBENCH_DIR` (`~/bootstrap`) gives `WORKTREES_DIR` and `SYSTEM_DIR`
(`~/bootstrap/system`), `SYSTEM_DIR` gives `SSH_DIR`, and `SSH_DIR`
gives `SSH_KEY`. A step that works in a place takes `--base FILE`, a
user's own base that sets only what it changes and is read first.
Nothing else comes from the environment but `HOME` and
`XDG_RUNTIME_DIR`. A step writes every file that names a place itself,
so the place is in the file as written.

## Environment

The caller enters the flake's default shell once and runs a step or
`dagu start` in it:

```shell
nix develop --ignore-env --keep-env-var HOME --keep-env-var XDG_RUNTIME_DIR
```

No step or DAG calls `nix develop`, and none uses a tool that happens to
be on the host's `PATH`. No step runs `sudo`, except
`steps/github/prepare_runner`, which does a GitHub runner's one-time root
setup and refuses to run anywhere else.

## DAGs

A DAG step's `id` is the step it runs, `<subsystem>_<verb>`
(`host_check`, `ssh_generate_key`), followed by the value of the
argument that tells two runs of that step in one DAG apart
(`cloud_fetch_debian`).

A DAG (`dags/<guide>.yaml`) is `type: chain`, takes no parameters, and
holds the control flow: it retries `machine/wait` until the guest
answers, and stops the machines from `handler_on: exit`, whether the run
passed or failed. Every run writes to the same places, so the DAGs that
start the `bootstrap` machine join the `bootstrap` queue in
`config.yaml`, which admits one at a time under `dagu enqueue`;
`dagu start` ignores it.

## CI

A workflow under `.github/workflows/` prepares the runner, then runs the
`dagu start` and `dagu status` a person runs, and keeps Dagu's logs. Its
matrix holds every DAG, and every push runs them all. Dagu is built by
`nix/dagu.nix` until its patches land upstream.

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
