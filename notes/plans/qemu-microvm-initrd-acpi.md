<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# File QEMU's microvm initrd-over-ACPI bug

Status: planned, 2026-10-28. The report below is drafted and not filed.
Once it has an issue number, cite it in `docs/bootstrap/direct.rst`'s
microvm step, which explains the cause from the source and pads the
initrd as the workaround, and in `steps/direct/pad_initrd`, which does
the same for the direct DAG.

## The draft

For <https://gitlab.com/qemu-project/qemu/-/issues>, titled "microvm:
initrd overwrites the ACPI tables when its size mod 4096 is large".

### Host environment

- Operating system: Debian (forky/sid), host kernel 6.18.15+deb14-amd64
- Architecture: x86_64
- QEMU flavor: qemu-system-x86_64
- QEMU version: 11.1.1 (Debian 1:11.1.1+ds-1) and 11.1.2 (nixpkgs); the
  code below is unchanged on master at bddb785e79
- QEMU command line:

  ```shell
  qemu-system-x86_64 -machine microvm -accel kvm -cpu host -m 2G \
      -nographic -serial stdio -monitor none \
      -kernel /boot/vmlinuz-7.2.6+deb14-amd64 -initrd junk.img \
      -append console=ttyS0
  ```

### Emulated/Virtualized environment

- Operating system: any Linux guest booted with `-kernel`/`-initrd`
  (Debian 7.2.6, NixOS 7.3)
- Architecture: x86_64

### Description of problem

With ACPI enabled (the microvm default), the guest hangs at boot after

```text
ACPI: Core revision 20260408
```

depending only on the size of the initrd and the number of virtio-mmio
devices. The initrd's contents do not matter.

microvm loads the kernel with no space reserved for ACPI data:

```c
/* hw/i386/microvm.c, microvm_memory_init() */
mmc->x86_load_linux(x86ms, fw_cfg, 0);
```

so `x86_load_linux()` in `hw/i386/x86-common.c` caps the initrd at
`below_4g_mem_size - 0 - 1` and places it at
`(initrd_max - initrd_size) & ~4095`. The initrd therefore ends inside
the top page of low RAM, up to `size % 4096` bytes into it. That page is
where the firmware (`bios-microvm.bin`) installs the ACPI tables from
`etc/acpi/tables`, and e820 marks it reserved (`0x7ffff000-0x7fffffff`
with `-m 2G`). The initrd is copied there after the tables, so any table
below the initrd's end is overwritten. The pc machines pass
`PC_FW_DATA` (`0x28000`) instead of 0 and are not affected.

The tables sit at the top of that page, and the DSDT grows by 0x44
bytes per virtio-mmio device, so the threshold falls as devices are
added. With `-m 2G`:

| virtio-mmio devices | DSDT start |
| --- | --- |
| 0 | 0x7ffffd00 (Debian kernel: 0x7ffffd40) |
| 1 | 0x7ffffcc0 |
| 3 | 0x7ffffc40 |
| 4 | 0x7ffffc00 |
| 5 | 0x7ffffbc0 |
| 12 | 0x7ffff9c0 |

An initrd hangs the guest once `size % 4096` passes the DSDT's offset in
that page. A NixOS initrd of 27,601,944 bytes (`% 4096 = 0xc18`) boots
with 3 devices and hangs with 4; one of 27,604,112 bytes
(`% 4096 = 0x490`) still boots with 12. Padding the hanging one with
zeros to a multiple of 4096 makes it boot with any device count. In the
hanging boots the kernel's table list has no DSDT line; booting ones
print, for example:

```text
RAMDISK: [mem 0x7e5ad000-0x7fffffff]
ACPI: XSDT 0x000000007FFFFFC3 000034 (v01 BOCHS  BXPC ...)
ACPI: DSDT 0x000000007FFFFC40 00021D (v02 BOCHS  BXPC ...)
```

### Steps to reproduce

Debian kernel and QEMU, a random-byte initrd (the kernel panics later
for want of a root file system, which marks a boot that got past ACPI):

1. `head --bytes=$((4096*1000 + 0xc18)) /dev/urandom >junk.img`, run
   the command line above: the guest prints `ACPI: DSDT
   0x000000007FFFFD40` and reaches `VFS: Unable to mount root fs`.
2. Add four devices, `-netdev user,id=nN -device
   virtio-net-device,netdev=nN` for N = 0..3: the guest stops at
   `ACPI: Core revision 20260408`.
3. Same four devices, `head --bytes=$((4096*1000))`: boots to the
   panic again, DSDT at 0x7FFFFC00.
4. No devices, `head --bytes=$((4096*1000 + 0xf00))`: stops at `ACPI:
   Core revision`.
5. No devices, `head --bytes=$((4096*1000 + 0xfff))`: the kernel prints
   nothing after the firmware's `Booting from ROM...`.

### Additional information

Passing a reserve for ACPI data from `microvm_memory_init()`, as
`pc_memory_init()` does with `PC_FW_DATA`, keeps the initrd out of the
tables' page; reserving at least one page when ACPI is enabled would be
enough for the tables microvm builds today. Untested. Workaround: pad
the initrd to a multiple of 4096 bytes (`truncate --size=%4096`).
