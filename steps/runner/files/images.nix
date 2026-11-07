# SPDX-License-Identifier: copyleft-next-0.3.1
let
  fetch = import <nix/fetchurl.nix>;
in
{
  # Ubuntu 26.04's cloud image stands in for the Azure image GitHub's
  # ubuntu-latest starts from.
  ubuntu = fetch {
    url = "https://cloud-images.ubuntu.com/releases/resolute/release-20261016/ubuntu-26.04-server-cloudimg-amd64.img";
    hash = "sha256-VkXQMYrSH2Q+uxrEVnUUYg6YkzgLB7AgH9oWO6pbfrU=";
  };
  # The runner-images release that built ubuntu-latest.
  runner-images = fetch {
    url = "https://github.com/actions/runner-images/archive/refs/tags/ubuntu26/20261026.180.tar.gz";
    hash = "sha256-FwinPJOJaoSqh8eKBtpygXYyY2nT3W8dFS1AraGAsBQ=";
  };
}
