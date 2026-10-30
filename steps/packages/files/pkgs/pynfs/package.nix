# SPDX-License-Identifier: copyleft-next-0.3.1
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
