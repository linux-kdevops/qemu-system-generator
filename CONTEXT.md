<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# qemu-system-generator automation

How the project reproduces its own workflows: what a person reads, what a
person runs, and what runs it for them.

## Language

**Guide**:
A page of `docs/bootstrap/`, written for a person to understand what they
run and why.
_Avoid_: recipe, script

**Step**:
A bash executable under `steps/<subsystem>/<verb>`: the commands a person
would type to do what a guide teaches, on the happy path, with as few
options as possible.
_Avoid_: task, action, script

**DAG**:
A file under `dags/` that wires steps into a run for Dagu, one per guide
and named after it. Control flow lives here, in Dagu's own terms:
retries, preconditions, loops, handlers.
_Avoid_: flow, pipeline, workflow

**Workflow**:
A file under `.github/workflows/` that prepares a GitHub runner and starts
a DAG.
_Avoid_: pipeline, job

**Workbench**:
The whole area the steps work in (`WORKBENCH_DIR`, default `~/bootstrap`).
_Avoid_: workspace, bootstrap directory

**Worktree-group**:
One guide's directory of work (`~/bootstrap/kernel`), under
`WORKTREES_DIR`, which defaults to `WORKBENCH_DIR`.
_Avoid_: guide directory, workbench

**System workbench**:
What every worktree-group shares (`SYSTEM_DIR`, default `WORKBENCH_DIR`):
the SSH key and configuration under `ssh/` (`SSH_DIR`, `SSH_KEY`) and the
directory the guests share, `share/`.
_Avoid_: ssh directory

**Base**:
The file every step reads for the places, each defaulting from the one
above it, so setting one moves those below it unless they are set too.
The repository's `steps/base` holds the defaults; a user's own base,
given with `--base FILE`, sets only what it changes.
_Avoid_: config, env file, profile

**Workload type**:
A kind of run named by the host resource it owns (the `bootstrap`
machine); runs of one type are admitted one at a time, runs of different
types may overlap. Carried by a Dagu queue.
_Avoid_: lock

## Relationships

- A **Guide** and its **Steps** are independent documents, kept by hand.
  Neither reads, includes or names the other; a file may be shared, but
  need not be.
- A **DAG** runs **Steps**; a **Workflow** starts one **DAG**.
- Every run writes to the same places: the **Workbench**,
  `~/.config/qemu/systemd` and `~/.local/state/qemu-system`, so two runs
  of one **Workload type** never overlap.
