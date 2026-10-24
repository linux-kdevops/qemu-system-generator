# SPDX-License-Identifier: copyleft-next-0.3.1
#
# Dagu from its main branch, with the fixes kdevops-ng-minimal-dagu carries
# until they land upstream; nix/dagu-patches/ is its copy of them.
{
  buildGo127Module,
  fetchFromGitHub,
  fetchPnpmDeps,
  nodejs,
  pnpm_10,
  pnpmConfigHook,
  stdenv,
}:
let
  version = "2.21.0-unstable-2026-10-19";
  src = fetchFromGitHub {
    owner = "dagucloud";
    repo = "dagu";
    rev = "92783b429b5569a92cae61421cb9eceb990eea73";
    hash = "sha256-sYr/nLDa3pEmP7+Gvs+unh8/aYI9XePl1o3V/fs4MkE=";
  };
  ui = stdenv.mkDerivation (finalAttrs: {
    pname = "dagu-ui";
    inherit version src;
    sourceRoot = "${finalAttrs.src.name}/ui";
    nativeBuildInputs = [
      nodejs
      pnpm_10
      pnpmConfigHook
    ];
    pnpmDeps = fetchPnpmDeps {
      inherit (finalAttrs)
        pname
        version
        src
        sourceRoot
        ;
      pnpm = pnpm_10;
      fetcherVersion = 4;
      hash = "sha256-SmJUgZcAXlqyS9nRzH+rytScvAHAYTXcy0H6QniB7FU=";
    };
    buildPhase = ''
      runHook preBuild
      NODE_OPTIONS=--max-old-space-size=8192 pnpm exec webpack --config webpack.prod.js
      runHook postBuild
    '';
    installPhase = ''
      cp --recursive dist $out
    '';
  });
in
buildGo127Module {
  pname = "dagu";
  inherit version src;
  # Branch dagomez/integration-fixes, the commits that change the binary:
  # step redirects to a terminal, and dagu start showing the running step
  # and its output.
  patches = [
    ./dagu-patches/0001-Skip-fsync-for-a-step-redirect-that-is-not-a-regular.patch
    ./dagu-patches/0002-Write-a-step-redirect-through-when-it-is-not-a-regul.patch
    ./dagu-patches/0003-Copy-each-step-s-output-to-an-optional-tap.patch
    ./dagu-patches/0004-Show-the-running-step-and-its-output-in-dagu-start.patch
    ./dagu-patches/0005-Show-five-lines-of-each-step-s-output-in-a-run-summa.patch
    ./dagu-patches/0006-Show-a-step-s-merged-output-once-in-a-run-summary.patch
  ];
  vendorHash = "sha256-jF3LU7RzS54qsazg28NVefJehm6EPLFmiE22O8iv+v4=";
  subPackages = [ "cmd" ];
  ldflags = [ "-X main.version=${version}" ];
  preBuild = ''
    cp ${ui}/* internal/service/frontend/assets/
  '';
  doCheck = false;
  postInstall = ''
    mv $out/bin/cmd $out/bin/dagu
  '';
  meta.mainProgram = "dagu";
}
