.. SPDX-License-Identifier: copyleft-next-0.3.1

:html_theme.sidebar_secondary.remove: true

=======================
From APT and DNF to Nix
=======================

The package commands of Debian and Ubuntu (:cmd:`apt`, :cmd:`dpkg-query`)
and of Fedora (:cmd:`dnf`, :cmd:`rpm`), beside the `Nix`_ commands that
do the same. ``fio`` stands for any package.

APT and DNF change the running system as root,
from the repositories in their lists. Nix builds or fetches each
package into a path of its own under ``/nix/store`` and changes which
paths a profile links to; a reference such as ``nixpkgs#fio`` names
where the package comes from, and nothing leaves the store until garbage
collection. The NixOS guests of these guides change nothing in place:
their packages come from ``configuration.nix``, as the
:ref:`second table <cheatsheet-guest>` shows.

On any machine with Nix
=======================

.. list-table::
   :class: cheatsheet
   :header-rows: 1
   :widths: 19 27 27 27

   * - Task
     - Debian, Ubuntu
     - Fedora
     - Nix
   * - Search
     - ``apt search fio``
     - ``dnf search fio``
     - ``nix search nixpkgs fio``
   * - Show the version and description
     - ``apt show fio``
     - ``dnf info fio``
     - ``nix search nixpkgs '^fio$'``
   * - Use once, without installing
     - None
     - None
     - ``nix shell nixpkgs#fio``

       ``nix run nixpkgs#fio -- --version``
   * - Install for your user
     - None
     - None
     - ``nix profile add nixpkgs#fio``
   * - Install for the whole system
     - ``sudo apt install fio``
     - ``sudo dnf install fio``
     - On NixOS, in ``configuration.nix``: :ref:`the second table
       <cheatsheet-guest>`
   * - Remove
     - ``sudo apt remove fio``
     - ``sudo dnf remove fio``
     - ``nix profile remove fio``
   * - Refresh the lists and upgrade everything
     - ``sudo apt update``

       ``sudo apt upgrade``
     - ``sudo dnf upgrade --refresh``
     - ``nix profile upgrade --all``
   * - List what you installed
     - ``apt-mark showmanual``
     - ``dnf repoquery --userinstalled``
     - ``nix profile list``
   * - List everything installed
     - ``dpkg-query --list``
     - ``rpm --query --all``
     - ``nix path-info --recursive ~/.nix-profile``
   * - List a package's files
     - ``dpkg-query --listfiles fio``
     - ``rpm --query --list fio``
     - ``nix store ls --recursive
       $(nix build --no-link --print-out-paths nixpkgs#fio)``
   * - Find the installed package that owns a file
     - ``dpkg-query --search /usr/bin/fio``
     - ``rpm --query --file /usr/bin/fio``
     - ``nix path-info "$(readlink --canonicalize "$(command -v fio)")"``
   * - Find the package that provides a file
     - ``apt-file search --fixed-string /usr/bin/fio``
     - ``dnf provides /usr/bin/fio``
     - ``nix-locate --whole-name --at-root /bin/fio``
   * - Show a package's dependencies
     - ``apt depends fio``
     - ``dnf repoquery --requires fio``
     - ``nix-store --query --references
       $(nix build --no-link --print-out-paths nixpkgs#fio)``
   * - Show what needs a package
     - ``apt rdepends --installed fio``

       ``apt why fio``
     - ``dnf repoquery --installed --whatrequires=fio``
     - ``nix why-depends ~/.nix-profile nixpkgs#fio``
   * - Get the source
     - ``apt source fio``
     - ``dnf download --srpm fio``
     - ``nix build --out-link fio-src nixpkgs#fio.src``
   * - Get the build dependencies
     - ``sudo apt build-dep fio``
     - ``sudo dnf builddep fio``
     - ``nix develop nixpkgs#fio``
   * - Hold a package at its version
     - ``sudo apt-mark hold fio``
     - ``sudo dnf versionlock add fio``
     - ``nix profile add github:NixOS/nixpkgs/<commit>#fio``, which
       ``nix profile upgrade`` leaves alone
   * - Add a repository
     - A file in ``/etc/apt/sources.list.d/``
     - ``sudo dnf config-manager addrepo
       --from-repofile=<url>``
     - ``nix profile add github:<owner>/<repo>#fio``
   * - Clean the cache and free disk space
     - ``sudo apt clean``

       ``sudo apt autoremove``
     - ``sudo dnf clean all``

       ``sudo dnf autoremove``
     - ``nix store gc``
   * - Check installed files
     - ``dpkg --verify fio``
     - ``rpm --verify fio``
     - ``nix store verify --recursive ~/.nix-profile``
   * - Undo the last change
     - ``sudo apt history-undo <id>``
     - ``sudo dnf history undo last``
     - ``nix profile rollback``
   * - Show the changelog
     - ``apt changelog fio``
     - ``dnf changelog fio``
     - ``nix eval --raw nixpkgs#fio.meta.changelog``

Debian 13 ships APT 3.0: ``apt why`` needs APT 3.1, and
``apt history-undo`` APT 3.1.7. ``apt-file`` is a package of its own,
with a file list that ``sudo apt update`` fetches once it is installed.
The Fedora commands are DNF5's, Fedora 41's and later; ``builddep``,
``config-manager`` and ``changelog`` come from the ``dnf5-plugins``
package. ``nix-locate`` comes from nix-index, whose ``nix-index``
command first builds the list it reads, from what the binary cache holds.

``nix search nixpkgs`` and ``nixpkgs#fio`` take nixpkgs from the flake
registry, the latest ``nixpkgs-unstable``, whose ``fio`` can be newer
than the one in your profile. ``apt source`` applies Debian's patches;
``nixpkgs#fio.src`` is upstream's source, before nixpkgs' ``patches``. A
meta attribute such as ``changelog`` links upstream's notes where the
recipe sets one; nixpkgs' own history of a package is the Git log of the
file that ``nix eval --raw nixpkgs#fio.meta.position`` names.

.. _cheatsheet-guest:

In a guest of these guides
==========================

A guide's guest is a flake on the host, ``~/bootstrap/direct`` in
:doc:`direct`: run these there. A change to its packages is an edit, a
:cmd:`nix build` and a :ref:`restart <host-start>`.

.. list-table::
   :class: cheatsheet
   :header-rows: 1
   :widths: 30 70

   * - Task
     - In ``~/bootstrap/direct``
   * - Install
     - Add ``pkgs.fio`` to ``environment.systemPackages`` in
       ``configuration.nix``, with ``pkgs`` among its arguments
       (:doc:`packages`), then build and restart.
   * - Remove
     - Delete it from ``environment.systemPackages``, then build and
       restart.
   * - Upgrade everything
     - ``nix flake update``, then build and restart
       (:ref:`packages-update`).
   * - Hold an input
     - ``flake.lock`` holds every input; ``nix flake update nixpkgs``
       moves that one alone.
   * - Add packages from elsewhere
     - An input in ``flake.nix`` and an overlay (:doc:`packages`).
   * - Show a package's version
     - ``nix eval --raw .#nixosConfigurations.bootstrap.pkgs.fio.version``
   * - List what you installed
     - ``nix eval .#nixosConfigurations.bootstrap.config.environment.systemPackages
       --apply 'map (p: p.name)'``
   * - List everything installed
     - ``nix path-info --recursive ./result``
   * - Show what needs a package
     - ``nix why-depends ./result .#nixosConfigurations.bootstrap.pkgs.fio``
   * - Find the package that owns a file
     - ``ssh bootstrap-direct readlink --canonicalize
       /run/current-system/sw/bin/fio``
   * - Check installed files
     - ``nix store verify --recursive ./result``
   * - Undo the last change
     - In a Git repository (:doc:`packages` shows how), revert the
       change, then build and restart.

.. _Nix: https://nix.dev/manual/nix/latest/
