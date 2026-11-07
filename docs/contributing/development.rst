.. SPDX-License-Identifier: copyleft-next-0.3.1

===========
Development
===========

The generator is Rust with no dependency outside the standard library; the
pages, the steps and the DAGs come with the flake's tools.

Rust
====

``#![forbid(unsafe_code)]``, ``clippy::pedantic`` clean with
``--deny warnings``, and ``cargo fmt``. Errors are enums with ``Display`` and
``Error::source()``, never strings. Newtypes (``UnitName``, ``Tag``) validate
in ``FromStr``. A value bound for unit text stays a ``String``; a path the
generator itself opens is a ``PathBuf``.

Never fabricate a fact. Every QEMU option, virtiofsd option, systemd
directive, specifier, path and device-node name is checked against its manual,
``--help`` or its source tree before it is written into code, docs or an
example.

Checks
======

Run these from the flake's development shell before committing:

.. code-block:: console
   :class: cmd-host

   $ cargo fmt --check
   $ cargo clippy --all-targets -- --deny warnings
   $ cargo test
   $ tests/verify
   $ nix flake check
   $ scripts/check-style.sh

``tests/verify`` generates the examples and loads them with
:cmd:`systemd-analyze(1)` ``verify`` in both scopes. ``tests/expected/`` is the
format's conformance record: never edit it by hand, but run
``tests/update-expected`` after a deliberate format change and review the
diff. :cmd:`nix flake check` also builds this site with warnings as errors,
holds the prose to 80 columns, and runs nixfmt, statix and deadnix over the
Nix files. ``scripts/check-style.sh`` checks trailing whitespace, the newline
at the end of each file, the SPDX headers, and HEAD's trailers and prose
(:doc:`writing`).

.. _commit-rules:

Commits
=======

A commit makes one change; a spelling fix does not ride along with code. Its
subject is ``subsystem: summary`` in the imperative, at most 75 characters,
the subsystem naming the area changed: ``generator``, ``backend``,
``launcher``, ``unitfile``, ``search``, ``plan``, ``examples``, ``tests``,
``docs``, ``designs``, ``nix``, ``scripts``, ``steps``, ``dags``, ``github``,
``README``, ``AGENTS.md``, or ``qemu-system-generator`` for a change across
the tree. Its body is plain paragraphs wrapped at 75 columns, with no list
(:doc:`writing`).

The message closes with a ``Signed-off-by`` in your Git identity, a
person's, who answers for the change under the Developer Certificate of
Origin. No trailer marks the use of a tool, and ``scripts/check-style.sh``
refuses ``Assisted-by``, ``Generated-by`` and ``Co-authored-by``: the kernel
dropped ``Assisted-by`` in 1e035297ffda ("docs: remove Assisted-by
references"), and as its `generated-content.rst`_ asks, a cover letter says
how tools were used and a commit message names a tool that found the problem
it fixes. A co-author gets a ``Co-developed-by`` immediately followed by their
own ``Signed-off-by``, which only they can give, and the last sign-off is the
committer's:

.. code-block:: text

   subsystem: summarise the change in the imperative mood

   What the problem is, and why the change takes this shape.

   Co-developed-by: Co Author <co.author@example.org>
   Signed-off-by: Co Author <co.author@example.org>
   Signed-off-by: Your Name <your.name@example.org>

The trailer and prose checks read HEAD; ``scripts/commit-msg``, linked as the
commit-msg hook, runs them on the message before the commit lands. Link it
once by hand:

.. code-block:: console
   :class: cmd-host

   $ ln --symbolic --relative \
       "$(git rev-parse --show-toplevel)/scripts/commit-msg" \
       "$(git rev-parse --git-path hooks)/commit-msg"

Under a global ``core.hooksPath`` the command links the hook into that
directory, for every repository: point this repository at its own first.

.. _generated-content.rst: https://docs.kernel.org/process/generated-content.html
