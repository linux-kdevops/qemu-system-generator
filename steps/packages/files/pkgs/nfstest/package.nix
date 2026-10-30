# SPDX-License-Identifier: copyleft-next-0.3.1
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
