# SPDX-License-Identifier: copyleft-next-0.3.1
#
# act for the runner DAG, which copies it into an Ubuntu guest: patched so
# that a self-hosted job runs as on GitHub's runner, and built without cgo
# so that the binary needs no library from the store. 0.2.92 copies an
# action into a job twice, as its post step does, where nixpkgs' 0.2.89
# fails on go-git's read-only pack files; its go.mod wants Go 1.26.10,
# newer than nixpkgs' default Go.
{
  act,
  buildGo127Module,
  fetchFromGitHub,
}:
(act.override { buildGoModule = buildGo127Module; }).overrideAttrs (
  finalAttrs: old: {
    version = "0.2.92";
    src = fetchFromGitHub {
      owner = "nektos";
      repo = "act";
      tag = "v${finalAttrs.version}";
      hash = "sha256-Ai8DcpQHzXsjN7FRgXpH7str0nvXPPpMYdEL4pWABv0=";
    };
    vendorHash = "sha256-swbESCLfvFlfpzVDhECGbwmrs8g3GgFisLJXfraytro=";
    ldflags = [
      "-s"
      "-w"
      "-X main.version=${finalAttrs.version}"
    ];
    patches = (old.patches or [ ]) ++ [
      ./act-patches/0001-Run-host-steps-without-a-terminal.patch
      ./act-patches/0002-Ignore-unknown-fields-in-artifact-requests.patch
      ./act-patches/0003-Sign-artifact-URLs-without-base64-padding.patch
      ./act-patches/0004-Cap-a-matrix-s-parallel-jobs-at-concurrent-jobs.patch
    ];
    env = (old.env or { }) // {
      CGO_ENABLED = "0";
    };
  }
)
