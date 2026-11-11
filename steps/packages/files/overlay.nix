# SPDX-License-Identifier: copyleft-next-0.3.1
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

  xfsprogs = prev.xfsprogs.overrideAttrs (previousAttrs: {
    version =
      let
        date = start: length: prev.lib.substring start length inputs.xfsprogs.lastModifiedDate;
      in
      "0-unstable-${date 0 4}-${date 4 2}-${date 6 2}";
    src = inputs.xfsprogs;
    postConfigure = (previousAttrs.postConfigure or "") + ''
      cp include/install-sh install-sh
      patchShebangs install-sh
    '';
  });

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
