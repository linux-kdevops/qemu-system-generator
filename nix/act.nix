# SPDX-License-Identifier: copyleft-next-0.3.1
# The runner DAG copies act into an Ubuntu guest, so it is built without
# cgo. nixpkgs' 0.2.89 fails on go-git's read-only pack files when a post
# step copies an action into a job twice; 0.2.92's go.mod wants Go 1.26.10.
{
  act,
  buildGo127Module,
  fetchFromGitHub,
}:
(act.override { buildGoModule = buildGo127Module; }).overrideAttrs (
  finalAttrs: previousAttrs: {
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
    patches = (previousAttrs.patches or [ ]) ++ [
      ./act-patches/0001-Run-host-steps-without-a-terminal.patch
      ./act-patches/0002-Ignore-unknown-fields-in-artifact-requests.patch
      ./act-patches/0003-Sign-artifact-URLs-without-base64-padding.patch
      ./act-patches/0004-Cap-a-matrix-s-parallel-jobs-at-concurrent-jobs.patch
    ];
    env = (previousAttrs.env or { }) // {
      CGO_ENABLED = "0";
    };
  }
)
