<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Regenerate after a change in a drop-in directory

Status: research, 2026-10-28.

`qemu-system-generator.path` has one line, `PathChanged=%E/qemu/systemd`.
systemd watches that directory with inotify for `IN_CLOSE_WRITE`,
`IN_CREATE`, `IN_DELETE`, moves and attribute changes (`src/core/path.c`
in systemd), and inotify reports events only for the entries directly in
a watched directory. Saving `NAME.qemu` runs the generator; saving
`NAME.qemu.d/10-closure.conf` does not, so the guides regenerate by hand
after every drop-in change (`docs/bootstrap/host.rst`, Regenerating).
Creating a `.qemu.d` directory does trigger it, since the directory is
an entry of the watched one.

## The change

The generator already reads every `.qemu.d` directory on each run. It
would also write a drop-in for its own path unit, one `PathChanged=` per
drop-in directory it found:

    # qemu-system-generator.path.d/50-drop-ins.conf
    [Path]
    PathChanged=/home/user/.config/qemu/systemd/NAME.qemu.d
    PathChanged=/home/user/.config/qemu/systemd/cloud-.qemu.d

`systemd.generator(7)` lets a generator write `.d/*.conf` drop-ins. A
new `.qemu.d` is caught by the existing watch, and the run it triggers
adds a watch for it. The system scope does the same with
`/etc/qemu/systemd`.

## Open

- Whether an active path unit takes the new `PathChanged=` lines on
  `daemon-reload`, or needs a restart; the generator service ends with a
  reload (`data/qemu-system-generator.service`, `ExecStopPost=`).
- The user service writes into `%t/systemd/user` and deletes its own
  files there before each run; the path unit's drop-in must be among
  them, or a removed `.qemu.d` stays watched.
- Every search path the generator reads, not only the first, would need
  its drop-in directories watched.

## Verifying it

Edit `NAME.qemu.d/10-closure.conf` with no manual regenerate and see
`--dryrun` and the loaded unit follow; remove a `.qemu.d` and see its
watch go. Then drop the manual regenerate from `docs/bootstrap/host.rst`
and the guides, and every DAG still passes.
