.. SPDX-License-Identifier: copyleft-next-0.3.1

====================
Packages of your own
====================

A `NixOS`_ guest installs what `nixpkgs`_ builds. When a test needs a
package built another way (a newer release, a fix upstream has not
released, a tree of your own, a tool nixpkgs lacks), an overlay changes
the package set the guest is built from. The examples are storage and
file system test tools: `fio`_, `xfstests`_, `damo`_, `nfstest`_ and
`pynfs`_.

Start from the guest of :doc:`direct` after :ref:`direct-link`, where a
change to the guest is a rebuild and a restart. :doc:`disk` and
:doc:`uki` build the same ``nixosConfigurations.bootstrap`` and take the
same overlay.

How nixpkgs builds a package
============================

Each package in nixpkgs is a function in a file of its own. The
function's arguments are what the package builds with and the options it
offers; it returns a derivation, the recipe Nix builds. ``callPackage``
calls the function with each argument taken from the package set by name
(`callPackage`_ on nix.dev). Find fio's file in the nixpkgs your flake
locks:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix eval --raw --inputs-from . nixpkgs#fio.meta.position
   /nix/store/sxb2ijsbdhi8s1lvc6ph692nm84vms3y-source/pkgs/by-name/fi/fio/package.nix:93

The file begins:

.. code-block:: nix
   :caption: pkgs/by-name/fi/fio/package.nix

   {
     lib,
     stdenv,
     fetchFromGitHub,
     fetchpatch2,
     # ...
     withGnuplot ? false,
     gnuplot,
     # ...
   }:

   stdenv.mkDerivation (finalAttrs: {
     pname = "fio";
     version = "3.42";

     src = fetchFromGitHub {
       owner = "axboe";
       repo = "fio";
       tag = "fio-${finalAttrs.version}";
       hash = "sha256-v2A2mY0Lvoje632761urfR7h1KHVcGnVDaKOMjexqis=";
     };
     # ...

.. _packages-overlay:

The overlay
===========

The package set is a fixed point: every package takes its arguments from
the finished set, so a change to one package reaches every package that
takes it. An overlay is a function of two package sets that returns the
packages it adds or replaces (`Overlays`_ in the nixpkgs manual):

- ``prev`` is the set before the overlay. Take from it the package you
  change, and functions such as ``callPackage``, ``lib`` and the
  fetchers.
- ``final`` is the finished set, with every overlay applied. Take a
  package's dependencies from it.

Create ``~/bootstrap/direct/overlay.nix``, which changes nothing yet:

.. code-block:: nix
   :caption: ~/bootstrap/direct/overlay.nix

   final: prev: { }

The flake exports it as ``overlays.default``, the name that replaced the
older ``overlay`` output and that other flakes use.
:cmd:`nix flake check` only checks that each entry of ``overlays`` is a
function whose first argument is named ``final``, ``_final`` or ``_``.
The guest takes it through the `nixpkgs.overlays`_ option.
``self.nixosConfigurations.bootstrap.pkgs`` is the guest's package set,
overlay applied. Naming its fio in ``packages`` lets :cmd:`nix build`
build that fio without the rest of the system:

.. code-block:: diff
   :caption: ~/bootstrap/direct/flake.nix

   --- a/bootstrap/direct/flake.nix
   +++ b/bootstrap/direct/flake.nix
   @@ -4,13 +4,19 @@
      outputs =
        { self, nixpkgs }:
        {
   +      overlays.default = import ./overlay.nix;
   +
          nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
            modules = [
              ./configuration.nix
              ./virtiofs.nix
              ./home.nix
   +          { nixpkgs.overlays = [ self.overlays.default ]; }
            ];
          };
   -      packages.x86_64-linux.default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
   +      packages.x86_64-linux = {
   +        default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
   +        inherit (self.nixosConfigurations.bootstrap.pkgs) fio;
   +      };
        };
    }

The guest installs a package by listing it in
`environment.systemPackages`_. Take ``pkgs`` among the arguments of
``configuration.nix``, as :ref:`nixos-kernel` does:

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

   --- a/bootstrap/direct/configuration.nix
   +++ b/bootstrap/direct/configuration.nix
   @@ -1,7 +1,8 @@
   -{ modulesPath, ... }:
   +{ modulesPath, pkgs, ... }:
    {
      imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

   +  environment.systemPackages = [ pkgs.fio ];
      networking.hostName = "bootstrap";
      services.openssh.enable = true;
      users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub

.. _packages-override:

1. An option
============

fio's function takes ``withGnuplot ? false``; set to true, it puts
`gnuplot`_ on the ``PATH`` of fio's Python plotting scripts, such as
``fio2gnuplot``. ``override`` calls the function again with the
arguments you give it (`overriding`_ in the nixpkgs manual):

.. code-block:: nix
   :caption: ~/bootstrap/direct/overlay.nix

   final: prev: {
     fio = prev.fio.override { withGnuplot = true; };
   }

Build fio alone. The binary cache has gnuplot but not this fio, so Nix
fetches gnuplot and builds fio:

.. code-block:: console
   :class: cmd-host

   $ nix build .#fio
   $ grep 'gnuplot-[0-9]' result/bin/fio2gnuplot
   PATH=${PATH/':''/nix/store/1m2kyhnk3s8f01hjmvc3jwp6kn309v0d-gnuplot-6.0.5/bin'':'/':'}
   PATH='/nix/store/1m2kyhnk3s8f01hjmvc3jwp6kn309v0d-gnuplot-6.0.5/bin'$PATH

.. _packages-override-attrs:

2. An install step
==================

fio builds ``t/io_uring``, a benchmark that drives io_uring without fio's
engines, but ``make install`` does not install it. ``overrideAttrs``
changes the attributes the function passes to ``mkDerivation``. Passed a
function, ``overrideAttrs`` calls it with the recipe's own attributes
(``previousAttrs`` by convention), so the overlay can add to
``postInstall`` rather than replace it. ``or ""`` keeps the override
working if a later nixpkgs drops fio's ``postInstall``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/overlay.nix

   final: prev: {
     fio = (prev.fio.override { withGnuplot = true; }).overrideAttrs (previousAttrs: {
       postInstall = (previousAttrs.postInstall or "") + ''
         install --mode=755 t/io_uring $out/bin/fio-io-uring
       '';
     });
   }

.. code-block:: console
   :class: cmd-host

   $ nix build .#fio
   $ result/bin/fio-io-uring -h | head --lines=2
   result/bin/fio-io-uring [options] -- [filenames]
    -d <int>  : IO Depth, default 128

.. _packages-release:

3. A newer release
==================

nixpkgs has fio 3.42, and fio 3.43 is out. A release is a new ``version``
and a new ``src``. The recipe derives its tag from the version, but a
new ``version`` alone keeps 3.42's hash. Nix finds a source with that
hash in the store or the binary cache and builds 3.42's source under the
new name, and nixpkgs warns that the version was overridden without
``src``.

The function form with two arguments, ``finalAttrs: previousAttrs:``,
also gives the attributes as they end up after every override, so the
new ``src`` takes its tag from the version as the recipe's does. The new
hash is not known yet: leave it empty, the fake hash method of the
nixpkgs manual (`Updating source hashes`_):

.. code-block:: nix
   :caption: ~/bootstrap/direct/overlay.nix

   final: prev: {
     fio = (prev.fio.override { withGnuplot = true; }).overrideAttrs (
       finalAttrs: previousAttrs: {
         version = "3.43";
         src = prev.fetchFromGitHub {
           owner = "axboe";
           repo = "fio";
           tag = "fio-${finalAttrs.version}";
           hash = "";
         };
         postInstall = (previousAttrs.postInstall or "") + ''
           install --mode=755 t/io_uring $out/bin/fio-io-uring
         '';
       }
     );
   }

The build fails and prints the hash it got:

.. code-block:: console
   :class: cmd-host

   $ nix build .#fio
   ...
   error: hash mismatch in fixed-output derivation '/nix/store/gssjc8aq5mq4x84xjxng11imnhdb8qfh-source.drv':
            specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
               got:    sha256-5/3Q4ObLIxL2EukxGzujrN6n7kagjI8WYZYDkmYH4cE=
   ...

Put it in ``hash`` and build again. The build now stops in its patch
phase. An override keeps every attribute it does not name, ``patches``
among them, and :cmd:`nix log` shows that the fix nixpkgs carries for
3.42 is already in 3.43:

.. code-block:: console
   :class: cmd-host

   $ nix log .#fio | tail --lines=6
   applying patch /nix/store/v8dxgpm1yk1bk8ndqq5m02hyaj1jzyzv-a84eece62edd46c1f4c8047f1052ac6181fc8b3e.patch?full_index=1
   patching file backend.c
   Reversed (or previously applied) patch detected!  Assume -R? [n]
   Apply anyway? [n]
   Skipping patch.
   1 out of 1 hunk ignored -- saving rejects to file backend.c.rej

Drop the patch:

.. code-block:: diff
   :caption: ~/bootstrap/direct/overlay.nix

   --- a/bootstrap/direct/overlay.nix
   +++ b/bootstrap/direct/overlay.nix
   @@ -8,6 +8,7 @@ final: prev: {
            tag = "fio-${finalAttrs.version}";
            hash = "sha256-5/3Q4ObLIxL2EukxGzujrN6n7kagjI8WYZYDkmYH4cE=";
          };
   +      patches = [ ];
          postInstall = (previousAttrs.postInstall or "") + ''
            install --mode=755 t/io_uring $out/bin/fio-io-uring
          '';

.. code-block:: console
   :class: cmd-host

   $ nix build .#fio
   $ result/bin/fio --version
   fio-3.43

.. _packages-patch:

4. A patch
==========

fio's master holds commits 3.43 lacks, such as 6a73355e3a6d ("fio: add
FALLOC_FL_WRITE_ZEROES support to falloc"). nixpkgs fetches a published
patch with ``fetchpatch2``, and keeps a patch as a file when it cannot
be fetched, may vanish from its URL, or fixes a problem only nixpkgs has
(`Patches`_ in nixpkgs' guide to contributing packages).
``fetchpatch2`` needs full index hashes in the patch, which GitHub gives
when the URL ends in ``?full_index=1``; a patch with short ones, such as
cgit's, takes ``fetchpatch`` instead. The patch's hash comes from the
same fake hash method:

.. code-block:: diff
   :caption: ~/bootstrap/direct/overlay.nix

   --- a/bootstrap/direct/overlay.nix
   +++ b/bootstrap/direct/overlay.nix
   @@ -8,7 +8,13 @@ final: prev: {
            tag = "fio-${finalAttrs.version}";
            hash = "sha256-5/3Q4ObLIxL2EukxGzujrN6n7kagjI8WYZYDkmYH4cE=";
          };
   -      patches = [ ];
   +      patches = [
   +        (prev.fetchpatch2 {
   +          name = "fio-add-falloc-write-zeroes.patch";
   +          url = "https://github.com/axboe/fio/commit/6a73355e3a6d29747dce80bdf488a46b5adff85d.patch?full_index=1";
   +          hash = "sha256-NrHuBku5hv2w3cmoa8xCq6+UxzHWukMNOmmh1f0nUxA=";
   +        })
   +      ];
          postInstall = (previousAttrs.postInstall or "") + ''
            install --mode=755 t/io_uring $out/bin/fio-io-uring
          '';

.. code-block:: console
   :class: cmd-host

   $ nix build .#fio
   $ result/bin/fio --cmdhelp=fallocate | grep write_zeroes
                       : write_zeroes Use fallocate(..., FALLOC_FL_WRITE_ZEROES, ...)

A patch of your own goes in the same list as a path relative to
``overlay.nix``. :cmd:`git format-patch` writes one file per commit,
named after its subject:

.. code-block:: console
   :class: cmd-host

   $ git -C /path/to/fio format-patch --output-directory ~/bootstrap/direct/patches/fio fio-3.43..

.. code-block:: nix

   patches = [ ./patches/fio/0001-your-change.patch ];

.. note::

   ``~/bootstrap/direct`` is not a Git repository. If you keep it in one,
   a flake there sees only the files Git tracks, so add the files once:

   .. code-block:: console
      :class: cmd-host

      $ cd ~/bootstrap/direct
      $ git init
      $ echo result > .gitignore
      $ git add --all
      $ git commit --message="Start from the direct guide"

   and :cmd:`git add` each new file before the build that needs it:

   .. code-block:: console
      :class: cmd-host

      $ git add patches/fio/0001-your-change.patch

   The diffs in these guides are what :cmd:`git diff` prints, so
   ``git diff HEAD`` shows your changes since the last commit in the same
   form.

.. _packages-tree:

5. Your own tree
================

xfstests gains tests between releases, and a test you write runs from
your own clone. An input with ``flake = false`` fetches a source tree
that is not a flake and locks its revision in ``flake.lock``
(`flake inputs`_). Follow xfstests' ``for-next`` branch, and pass the
inputs to the overlay:

.. code-block:: diff
   :caption: ~/bootstrap/direct/flake.nix

   --- a/bootstrap/direct/flake.nix
   +++ b/bootstrap/direct/flake.nix
   @@ -1,10 +1,16 @@
    {
   -  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
   +  inputs = {
   +    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
   +    xfstests = {
   +      url = "git+https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git?ref=for-next";
   +      flake = false;
   +    };
   +  };

      outputs =
   -    { self, nixpkgs }:
   +    { self, nixpkgs, ... }@inputs:
        {
   -      overlays.default = import ./overlay.nix;
   +      overlays.default = import ./overlay.nix inputs;

          nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
            modules = [

``overlay.nix`` now takes the inputs first and returns the overlay. The
version takes nixpkgs' form for a commit (`Versioning`_), ``0-unstable-``
and the date of the input's last commit, with ``0`` because the overlay
cannot tell which release a moving branch follows. The fix for glibc
2.44 that nixpkgs carries is in ``for-next`` already:

.. code-block:: diff
   :caption: ~/bootstrap/direct/overlay.nix

   --- a/bootstrap/direct/overlay.nix
   +++ b/bootstrap/direct/overlay.nix
   @@ -1,4 +1,4 @@
   -final: prev: {
   +inputs: final: prev: {
      fio = (prev.fio.override { withGnuplot = true; }).overrideAttrs (
        finalAttrs: previousAttrs: {
          version = "3.43";
   @@ -20,4 +20,14 @@ final: prev: {
          '';
        }
      );
   +
   +  xfstests = prev.xfstests.overrideAttrs {
   +    version =
   +      let
   +        date = start: length: prev.lib.substring start length inputs.xfstests.lastModifiedDate;
   +      in
   +      "0-unstable-${date 0 4}-${date 4 2}-${date 6 2}";
   +    src = inputs.xfstests;
   +    patches = [ ];
   +  };
    }

Add ``xfstests`` to ``packages`` as fio is. xfstests' function takes
``fio`` from ``final``, so this xfstests runs the fio of the steps
above:

.. code-block:: console
   :class: cmd-host

   $ nix build .#xfstests
   warning: updating lock file "/home/user/bootstrap/direct/flake.lock":
   • Added input 'xfstests':
       'git+https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git?ref=for-next&rev=d07dca72d2455f4ba8c660ad4c008541ccccb707' (2026-10-14)
   $ grep --only-matching '[a-z0-9]*-fio-[0-9.]*' result/bin/xfstests-check
   f5qsjjzzlbsylsn4pi792h79cmd64kmp-fio-3.43

:cmd:`nix flake update xfstests <nix flake update>` moves the lock to the
branch's head. ``--override-input`` builds from your clone instead, with
the commit it has checked out and any change not yet committed, and
leaves ``flake.lock`` as it is:

.. code-block:: console
   :class: cmd-host

   $ nix build --override-input xfstests git+file:///path/to/xfstests-dev

.. _packages-new:

6. A new package
================

damo, the user space tool for the kernel's `DAMON`_, is not in nixpkgs. A
new package is a function like fio's, in a ``package.nix`` in a
directory named after it, as nixpkgs lays out ``pkgs/by-name``
(`by-name`_). Create ``~/bootstrap/direct/pkgs/damo/package.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/pkgs/damo/package.nix

   {
     lib,
     python3Packages,
     fetchFromGitHub,
   }:

   python3Packages.buildPythonApplication (finalAttrs: {
     pname = "damo";
     version = "3.4.3";
     pyproject = true;

     src = fetchFromGitHub {
       owner = "damonitor";
       repo = "damo";
       tag = "v${finalAttrs.version}";
       hash = "sha256-WFF9p6ENMfiqVm+Aa7uvpKAoq1Hnu9BH7UCQQLZbOCo=";
     };

     # Upstream keeps its modules flat in src/ and its packaging files in
     # packaging/; lay them out as packaging/build.sh does.
     postPatch = ''
       cp packaging/pyproject.toml packaging/setup.py .
       mkdir src/damo
       mv src/*.py src/damo/
       touch src/damo/__init__.py
     '';

     build-system = [ python3Packages.setuptools ];

     pythonImportsCheck = [ "damo" ];

     meta = {
       description = "User space tool for DAMON";
       homepage = "https://github.com/damonitor/damo";
       license = lib.licenses.gpl2Only;
       platforms = lib.platforms.linux;
       mainProgram = "damo";
     };
   })

With ``pyproject = true``, ``buildPythonApplication`` (`Python`_ in the
nixpkgs manual) builds a wheel with the packages in ``build-system``,
and ``pythonImportsCheck`` fails the build if the module does not
import.

``lib.packagesFromDirectoryRecursive`` calls ``callPackage`` on each
``package.nix`` under a directory and names the package after its
directory (`packagesFromDirectoryRecursive`_). Add the packages it
returns to the overlay's set; nixfmt then puts the set's opening brace on
a line of its own:

.. code-block:: diff
   :caption: ~/bootstrap/direct/overlay.nix

   --- a/bootstrap/direct/overlay.nix
   +++ b/bootstrap/direct/overlay.nix
   @@ -1,4 +1,5 @@
   -inputs: final: prev: {
   +inputs: final: prev:
   +{
      fio = (prev.fio.override { withGnuplot = true; }).overrideAttrs (
        finalAttrs: previousAttrs: {
          version = "3.43";
   @@ -31,3 +32,7 @@ inputs: final: prev: {
        patches = [ ];
      };
    }
   +// prev.lib.packagesFromDirectoryRecursive {
   +  inherit (prev) callPackage;
   +  directory = ./pkgs;
   +}

Add ``damo`` to ``packages``, and build it:

.. code-block:: console
   :class: cmd-host

   $ nix build .#damo
   $ result/bin/damo version
   3.4.3

.. _packages-more:

7. Two more
===========

nfstest ships as a release tarball. Its tests mount NFS and capture
packets, so ``makeWrapperArgs`` puts ``mount.nfs`` and ``tcpdump`` on the
``PATH`` of each installed script, without making them Python
dependencies. Create ``~/bootstrap/direct/pkgs/nfstest/package.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/pkgs/nfstest/package.nix

   {
     lib,
     python3Packages,
     fetchurl,
     nfs-utils,
     tcpdump,
   }:

   python3Packages.buildPythonApplication (finalAttrs: {
     pname = "nfstest";
     version = "3.2";
     pyproject = true;

     src = fetchurl {
       url = "https://www.linux-nfs.org/~mora/nfstest/releases/NFStest-${finalAttrs.version}.tar.gz";
       hash = "sha256-TUxcWttygx9Xhyde8jxUFP9cV84SDK+xzVPeB2ZXRcE=";
     };

     # setup.py imports distutils, which Python 3.12 removed, and builds the
     # man pages by running scripts it has not installed yet; the release
     # tarball ships them built. A relative man directory lands under $out.
     postPatch = ''
       substituteInPlace setup.py \
         --replace-fail 'from distutils.core import setup' 'from setuptools import setup' \
         --replace-fail 'from distutils.command.build import build' 'from setuptools.command.build import build' \
         --replace-fail 'create_manpage.run()' 'pass'
       substituteInPlace nfstest_config.py \
         --replace-fail "'/usr/share/man'" "'share/man'"
     '';

     build-system = [ python3Packages.setuptools ];

     makeWrapperArgs = [
       "--prefix"
       "PATH"
       ":"
       (lib.makeBinPath [
         nfs-utils
         tcpdump
       ])
     ];

     pythonImportsCheck = [ "nfstest" ];

     meta = {
       description = "NFS test suite";
       homepage = "https://www.linux-nfs.org/~mora/nfstest/";
       license = lib.licenses.gpl2Only;
       platforms = lib.platforms.linux;
       mainProgram = "nfstest_posix";
     };
   })

Python 3.13 removed xdrlib, so pynfs needs the xdrlib3 fork, which
nixpkgs lacks. A Python library belongs in the Python package sets, and
``pythonPackagesExtensions`` adds to every one of them
(`Python extensions`_). Create
``~/bootstrap/direct/python-packages/xdrlib3/package.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/python-packages/xdrlib3/package.nix

   {
     lib,
     buildPythonPackage,
     fetchPypi,
     poetry-core,
   }:

   buildPythonPackage (finalAttrs: {
     pname = "xdrlib3";
     version = "0.1.1";
     pyproject = true;

     src = fetchPypi {
       inherit (finalAttrs) pname version;
       hash = "sha256-JmlwE69Fr6tsb/mlmP0El5zeg/pyFdoFlVGwiK6ojzQ=";
     };

     build-system = [ poetry-core ];

     pythonImportsCheck = [ "xdrlib3" ];

     meta = {
       description = "Fork of the xdrlib module that Python 3.13 removed";
       homepage = "https://github.com/overcat/xdrlib3";
       license = lib.licenses.asl20;
     };
   })

Each Python package set has its own ``callPackage``, which takes
arguments such as ``buildPythonPackage`` from that set. Only
``python-final`` has it:

.. code-block:: diff
   :caption: ~/bootstrap/direct/overlay.nix

   --- a/bootstrap/direct/overlay.nix
   +++ b/bootstrap/direct/overlay.nix
   @@ -31,6 +31,16 @@ inputs: final: prev:
        src = inputs.xfstests;
        patches = [ ];
      };
   +
   +  pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
   +    (
   +      python-final: python-prev:
   +      prev.lib.packagesFromDirectoryRecursive {
   +        inherit (python-final) callPackage;
   +        directory = ./python-packages;
   +      }
   +    )
   +  ];
    }
    // prev.lib.packagesFromDirectoryRecursive {
      inherit (prev) callPackage;

Each test server runs under a Python with the modules it imports,
without setuptools and ply, which only build pynfs. The source is the
0.5 tag of the repository the `pynfs`_ wiki links. Create
``~/bootstrap/direct/pkgs/pynfs/package.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/pkgs/pynfs/package.nix

   {
     lib,
     stdenv,
     fetchFromGitHub,
     python3,
     makeWrapper,
   }:

   let
     python = python3.withPackages (ps: [
       ps.gssapi
       ps.xdrlib3
     ]);
     # setup.py build also imports nfs4.0's test server, for its version.
     buildPython = python3.withPackages (ps: [
       ps.gssapi
       ps.ply
       ps.setuptools
       ps.xdrlib3
     ]);
   in
   stdenv.mkDerivation (finalAttrs: {
     pname = "pynfs";
     version = "0.5";

     src = fetchFromGitHub {
       owner = "linux-nfs";
       repo = "pynfs";
       tag = "pynfs-${finalAttrs.version}";
       hash = "sha256-UiPJ6VKJ6sB+g+xUdmPdxUx3iIvU/rk3hMH7MScff+Y=";
     };

     nativeBuildInputs = [
       makeWrapper
       buildPython
     ];

     # pynfs runs from its own tree, where setup.py build generates the XDR
     # modules each test server imports; its use_local.py finds rpc and xdr
     # from the working directory, so the wrappers name them instead.
     buildPhase = ''
       runHook preBuild
       python3 setup.py build
       runHook postBuild
     '';

     installPhase = ''
       runHook preInstall
       rm --recursive */build
       mkdir --parents $out/share $out/bin
       cp --recursive . $out/share/pynfs
       for v in 4.0 4.1; do
         makeWrapper ${python.interpreter} $out/bin/nfs$v-testserver \
           --add-flags $out/share/pynfs/nfs$v/testserver.py \
           --prefix PYTHONPATH : $out/share/pynfs:$out/share/pynfs/xdr
       done
       runHook postInstall
     '';

     meta = {
       description = "NFSv4.0 and NFSv4.1 server conformance tests";
       homepage = "https://wiki.linux-nfs.org/wiki/index.php/Pynfs";
       license = lib.licenses.gpl2Only;
       platforms = lib.platforms.linux;
       mainProgram = "nfs4.1-testserver";
     };
   })

``packagesFromDirectoryRecursive`` finds both packages. Add ``nfstest``
and ``pynfs`` to ``packages``.

.. _packages-guest:

8. In the guest
===============

The files as they end up:

.. code-block:: nix
   :caption: ~/bootstrap/direct/flake.nix

   {
     inputs = {
       nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
       xfstests = {
         url = "git+https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git?ref=for-next";
         flake = false;
       };
     };

     outputs =
       { self, nixpkgs, ... }@inputs:
       {
         overlays.default = import ./overlay.nix inputs;

         nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
           modules = [
             ./configuration.nix
             ./virtiofs.nix
             ./home.nix
             { nixpkgs.overlays = [ self.overlays.default ]; }
           ];
         };
         packages.x86_64-linux = {
           default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
           inherit (self.nixosConfigurations.bootstrap.pkgs)
             damo
             fio
             nfstest
             pynfs
             xfstests
             ;
         };
       };
   }

.. code-block:: nix
   :caption: ~/bootstrap/direct/overlay.nix

   inputs: final: prev:
   {
     fio = (prev.fio.override { withGnuplot = true; }).overrideAttrs (
       finalAttrs: previousAttrs: {
         version = "3.43";
         src = prev.fetchFromGitHub {
           owner = "axboe";
           repo = "fio";
           tag = "fio-${finalAttrs.version}";
           hash = "sha256-5/3Q4ObLIxL2EukxGzujrN6n7kagjI8WYZYDkmYH4cE=";
         };
         patches = [
           (prev.fetchpatch2 {
             name = "fio-add-falloc-write-zeroes.patch";
             url = "https://github.com/axboe/fio/commit/6a73355e3a6d29747dce80bdf488a46b5adff85d.patch?full_index=1";
             hash = "sha256-NrHuBku5hv2w3cmoa8xCq6+UxzHWukMNOmmh1f0nUxA=";
           })
         ];
         postInstall = (previousAttrs.postInstall or "") + ''
           install --mode=755 t/io_uring $out/bin/fio-io-uring
         '';
       }
     );

     xfstests = prev.xfstests.overrideAttrs {
       version =
         let
           date = start: length: prev.lib.substring start length inputs.xfstests.lastModifiedDate;
         in
         "0-unstable-${date 0 4}-${date 4 2}-${date 6 2}";
       src = inputs.xfstests;
       patches = [ ];
     };

     pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
       (
         python-final: python-prev:
         prev.lib.packagesFromDirectoryRecursive {
           inherit (python-final) callPackage;
           directory = ./python-packages;
         }
       )
     ];
   }
   // prev.lib.packagesFromDirectoryRecursive {
     inherit (prev) callPackage;
     directory = ./pkgs;
   }

Install the five packages in the guest:

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

   --- a/bootstrap/direct/configuration.nix
   +++ b/bootstrap/direct/configuration.nix
   @@ -2,7 +2,13 @@
    {
      imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

   -  environment.systemPackages = [ pkgs.fio ];
   +  environment.systemPackages = [
   +    pkgs.damo
   +    pkgs.fio
   +    pkgs.nfstest
   +    pkgs.pynfs
   +    pkgs.xfstests
   +  ];
      networking.hostName = "bootstrap";
      services.openssh.enable = true;
      users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub

:cmd:`nix flake check` evaluates every package in ``packages``, the
system among them, so an override that fails to evaluate fails here,
before anything builds.
Rebuild the system and :ref:`restart <host-start>` the machine:

.. code-block:: console
   :class: cmd-host

   $ nix flake check
   ...
   all checks passed!
   $ nix build

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-direct 'fio --version; damo version; nfstest_posix --version; nfs4.1-testserver --help 2>/dev/null | head --lines=1'
   fio-3.43
   3.4.3
   nfstest_posix 1.2
   Usage: testserver.py SERVER:/PATH [options] flags|testcodes

.. _packages-update:

9. Updates
==========

``flake.lock`` pins nixpkgs and xfstests to the commits the first build
took. :cmd:`nix flake update` moves both to the head of their branches:

.. code-block:: console
   :class: cmd-host

   $ nix flake update
   warning: updating lock file "/home/user/bootstrap/direct/flake.lock":
   • Updated input 'nixpkgs':
       'github:NixOS/nixpkgs/e7439b6b14ad3cc35d05608ebca9bce01a25f5f8?narHash=sha256-H83yz/do%2BhjOO0uujHyH8RgtIQP2v7Mw%2BcLwcZK/xKc%3D' (2026-10-08)
     → 'github:NixOS/nixpkgs/31ecbaca1bdefa45dffc409f0250bd8039ea0010?narHash=sha256-dhoW6VgFWlaodSY5jrBn%2BCF2FLmaUvDVgM7%2BC7T2%2BNM%3D' (2026-10-27)
   $ nix build

:cmd:`nix build` builds the new closure and points ``result`` at it. The
machine's ``kernel=``, ``initrd=`` and ``init=`` all go through
``result``, so a :ref:`restart <host-start>` boots the new closure. The
packages the overlay changes, and the ones that take them, build on your
machine again, since the binary cache holds only nixpkgs' own builds.

An override keeps what it names across updates: fio stays at 3.43 with
its patch until you change the overlay. Once nixpkgs has what an
override adds, delete the override.

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _fio: https://github.com/axboe/fio
.. _xfstests: https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git
.. _damo: https://github.com/damonitor/damo
.. _nfstest: https://www.linux-nfs.org/~mora/nfstest/
.. _pynfs: https://wiki.linux-nfs.org/wiki/index.php/Pynfs
.. _DAMON: https://docs.kernel.org/admin-guide/mm/damon/index.html
.. _gnuplot: http://www.gnuplot.info
.. _callPackage: https://nix.dev/tutorials/callpackage
.. _Overlays: https://nixos.org/manual/nixpkgs/unstable/#chap-overlays
.. _overriding: https://nixos.org/manual/nixpkgs/unstable/#chap-overrides
.. _nixpkgs.overlays: https://nixos.org/manual/nixos/unstable/options#opt-nixpkgs.overlays
.. _environment.systemPackages: https://nixos.org/manual/nixos/unstable/options#opt-environment.systemPackages
.. _Updating source hashes: https://nixos.org/manual/nixpkgs/unstable/#sec-pkgs-fetchers-updating-source-hashes
.. _Patches: https://github.com/NixOS/nixpkgs/blob/master/pkgs/README.md#patches
.. _Versioning: https://github.com/NixOS/nixpkgs/blob/master/pkgs/README.md#versioning
.. _flake inputs: https://nix.dev/manual/nix/latest/command-ref/new-cli/nix3-flake#flake-inputs
.. _by-name: https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/README.md
.. _Python: https://nixos.org/manual/nixpkgs/unstable/#buildpythonapplication-function
.. _packagesFromDirectoryRecursive: https://nixos.org/manual/nixpkgs/unstable/#function-library-lib.filesystem.packagesFromDirectoryRecursive
.. _Python extensions: https://nixos.org/manual/nixpkgs/unstable/#how-to-override-a-python-package-for-all-python-versions-using-extensions
