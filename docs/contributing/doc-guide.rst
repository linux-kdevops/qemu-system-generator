.. SPDX-License-Identifier: copyleft-next-0.3.1

==========================
How to write documentation
==========================

This site is `reStructuredText`_ built with `Sphinx`_ and the
`PyData Sphinx Theme`_, following the kernel's conventions; what the prose
says is in :doc:`writing`. Keep the markup simple: ``::`` for a plain block,
``.. code-block:: <language>`` for one that needs highlighting, and double
backticks for inline literals. The man pages under ``man/`` are `scdoc`_; a
page here links them rather than repeating them.

License header
==============

Every page starts with an SPDX license identifier on line one, which
``scripts/check-style.sh`` looks for in the first two lines, then a blank line
and the title:

.. code-block:: rst

   .. SPDX-License-Identifier: copyleft-next-0.3.1

   ==========
   Page title
   ==========

Line length
===========

Wrap prose at 80 columns; a link target, whose URL has no space, is exempt.
:cmd:`nix flake check` enforces the limit with `doc8`_ (``doc8.ini``), which
skips directives, literal blocks and lines with no space, so keep code lines
short by hand, with a continuation.

Headings
========

Use the kernel's adornments: ``=`` with an overline for the title, ``=`` for
chapters, ``-`` for sections and ``~`` for subsections, in sentence case.
Upstream spellings such as QEMU, NixOS and virtiofsd keep their own case.

Names and identifiers
=====================

Name a project in plain text with its canonical spelling (QEMU, systemd,
virtiofsd, Nix, Cargo) and set the command you run as a literal
(``qemu-system-x86_64``, ``systemctl``, ``cargo``): "Cargo builds it", but
"run ``cargo test``". Set directives with their trailing ``=``
(``ExecStart=``), paths, options and file names as literals.

Write long options, ``--symbolic`` and not ``-s``, in every command shown,
unless the tool has no long form (``ssh -o``, ``ssh -i``).

Avoid the em and en dash; rewrite such a sentence as two, or use a colon, a
semicolon or parentheses.

Links
=====

Link the first mention on a page of an upstream project, tool, service or
systemd directive, or command with a manual page, to its manual or source;
later mentions stay plain. Cite QEMU, virtiofsd and systemd by their own
manuals (``man qemu-system``, ``virtiofsd --help``, ``man systemd.exec``). For
a link you reuse, or a long URL, define a named target at the foot of the
file:

.. code-block:: rst

   The site is built with `Sphinx`_.

   .. _Sphinx: https://www.sphinx-doc.org/

Link another page of this site with ``:doc:`` and a labelled section with
``:ref:`` rather than a hard-coded path.

The site's roles
----------------

``docs/conf.py`` registers these, each rendered as a literal:

``:cmd:`systemctl```
   A command, tool, manual page or directive, linked to the manual that the
   ``cmd_links`` table names; an unregistered name fails the build.
   ``:cmd:`KillMode=mixed <KillMode=>``` shows the title and links the target.
``:key:`SSHPrivateKey=```
   A key or section of the ``.qemu`` format, linked to where
   :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` defines it.
``:gh:`owner/repo/blob/main/file``` and ``:qemu:`blob/master/file```
   A file or commit on GitHub, or in QEMU's GitLab project.
``:line:`overlay-option:12```
   "line 12" of the code block named ``overlay-option``, which carries
   ``:name:`` and ``:linenos:``. A block's name must not equal a link target
   on the same page.

Host and guest blocks
=====================

A block of commands run on the host (a ``$`` prompt) carries
``:class: cmd-host``; one run on the guest, or of output the guest produced,
carries ``:class: cmd-guest``. The class draws a coloured rail from
``docs/_static/custom.css``. Caption only the first block of each kind on a
page. Where it runs decides, not the prompt. A configuration fragment or a
file's contents is neither.

Examples
========

An example is generic: placeholder paths under ``/path/to/``, relative image
names that resolve in the state directory, and no user name or home
directory. Not ``/srv/``: the UAPI file system hierarchy gives it to server
payload that may mount only late in boot.

Diffs in the walk-throughs
==========================

A walk-through's diff blocks are hunks from the commits of
`qemu-system-generator-bootstrap`_, one branch per guide: change a step
there, paste what its ``guides hunk`` prints, and run its ``guides check``
against this tree. A hand edit of a diff block, or of a captioned file block,
fails that check.

.. _reStructuredText: https://docutils.sourceforge.io/rst.html
.. _Sphinx: https://www.sphinx-doc.org/
.. _PyData Sphinx Theme: https://pydata-sphinx-theme.readthedocs.io/
.. _scdoc: https://git.sr.ht/~sircmpwn/scdoc
.. _doc8: https://github.com/PyCQA/doc8
.. _qemu-system-generator-bootstrap: https://github.com/linux-kdevops/qemu-system-generator-bootstrap
