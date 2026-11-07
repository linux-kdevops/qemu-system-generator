# SPDX-License-Identifier: copyleft-next-0.3.1
{ distro, version }:
let
  fetch = import <nix/fetchurl.nix>;
in
{
  debian."14" = fetch {
    url = "https://cloud.debian.org/images/cloud/forky/daily/20261102-2650/debian-14-genericcloud-amd64-daily-20261102-2650.qcow2";
    hash = "sha512-WkTKuUtV7IRcD39vQBE1ilGarS2FQcBlFejFfgTzCe9ENBna4FBquGRA9T6niXfj+urCBEFu4I6I+Xef1a63uQ==";
  };
  fedora."44" = fetch {
    url = "https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2";
    hash = "sha256-KGgP5bNxpaguv0OjGSbghqFo5ZlJ0DlpxQk+cHH5C38=";
  };
  ubuntu."26.10" = fetch {
    url = "https://cloud-images.ubuntu.com/stonking/20261105/stonking-server-cloudimg-amd64.img";
    hash = "sha256-nvIkOkMZ5DFkqxHWHjJO+g3RnthQl50Qw6bkT+tWhMU=";
  };
}
.${distro}.${version}
