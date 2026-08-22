# aiueos bare-metal integration

This directory contains the Linux-independent aiueos boot path owned by the
Kotoba product integration repository.

The current Phase 1 slice builds a PE32+ `BOOTX64.EFI` and a separate ELF64
`KERNEL.ELF`. OVMF starts the loader, which validates and places bounded ELF
segments, captures the firmware memory map, exits UEFI boot services, and
hands control to the kernel. The kernel validates the handoff and terminates
QEMU through the test-only debug-exit device. Its assembly entry switches to a
private 64 KiB stack before entering C, and its first hardware driver
initializes COM1 at 115200 baud. The kernel then installs its own GDT and IDT;
the smoke gate executes `ud2` and requires the vector 6 handler to terminate
QEMU. Before that test the kernel replaces the firmware CR3 with its own
four-level identity map, enables write-protect and NX, and maps text RX,
rodata R+NX, and writable state RW+NX. It does not use Linux, a JVM, GRUB, or a
host initramfs in the guest. The smoke test writes to text and attempts to
execute a byte in rodata; both must raise vector 14 with the expected x86 page
fault error-code bits before execution can continue.

The loader also selects the ACPI 2.0 configuration-table GUID. The kernel
validates both RSDP checksums, the XSDT and MADT checksums and lengths, and every
MADT subtable boundary. The QEMU gate starts two vCPUs and requires both to be
reported as enabled by MADT. The BSP then copies a real-mode trampoline below
1 MiB, sends INIT plus two SIPIs to the second MADT APIC ID, and requires the AP
to enter long mode on its own 64 KiB stack before the boot smoke may pass.

The BSP enables its Local APIC, maps the MMIO page cache-disabled, installs a
periodic timer on vector 32, enters `sti; hlt`, and requires the interrupt stub
to acknowledge EOI before the smoke test can continue.

The timer stub also preserves the complete x86-64 integer interrupt frame and
passes its stack pointer to a minimal round-robin scheduler. Two kernel tasks
run on separate 16 KiB stacks alongside the boot task. Each worker owns a
distinct CR3 and increments only its private page; the timer switch restores
the kernel CR3 before the boot task resumes. The QEMU gate proceeds
only after all three contexts have been preempted and both worker tasks have
resumed at least twice, producing `AIUEOS_SCHEDULER_OK` and
`AIUEOS_SCHEDULER_CR3_OK`. Interrupt and kernel mappings remain shared and
supervisor-only in every root.

The Phase 3 bootstrap installs an `int 0x80` syscall gate that preserves the
same integer context and returns through `iretq`. A tagged, generation-bearing
capability handle is required by the log-write admission path. The QEMU gate
proves that a stale generation, a non-canonical pointer, and a range crossing
the bootstrap mapping are denied before dereference. The process foundation reserves distinct U/S pages
for RX user text and RW+NX user data, leaves an unmapped guard page, and builds
a loaded 64-bit TSS descriptor with a dedicated kernel-entry stack. A one-shot
CPL3 task enters through `iretq`, exercises valid and rejected `int 0x80`
requests, and exits back to the kernel through the TSS `rsp0` path. Per-process
address-space groundwork then constructs two distinct CR3 roots. Each root
clones the low kernel page-table path, shares the kernel/MMIO branches, maps a
different private user page, and leaves the other process's page non-present.
The smoke switches CR3 sequentially, proves independent contents, and requires
real non-present page faults for both cross-process reads before restoring the
kernel CR3. Actual copy-in and the `syscall`/`sysret` transport remain later
work.

The PCI path performs a bounded configuration-space scan and validates modern
virtio vendor capabilities, including a cycle-limited capability chain, BAR
kind and width, and overflow-safe capability ranges. PCI MMIO is identity
mapped UC/NX only after validation, including QEMU's 64-bit MMIO window above
512 GiB. The virtio-rng smoke path negotiates `VIRTIO_F_VERSION_1`, allocates
separate zeroed pages for the descriptor, available ring, used ring, and data
buffer, submits one writable 32-byte request, and requires the device's used
ring completion. The same bounded capability parser also drives a modern-only
virtio-blk device. It reads the generation-stable capacity, rejects an empty or
overflowing device, submits a three-descriptor `VIRTIO_BLK_T_IN` chain, and
requires a 513-byte used completion, success status, and deterministic sector-0
identity. The smoke disk is a separate writable 1 MiB fixture, so neither the
ESP nor a release image can be modified by this gate. Sector 0 remains a
bounded read-only `aiuefs-v1` object store. Sector 1 is a dedicated journal
slot: boot first validates an existing committed record, including independent
header and payload checksums, and adopts it without
mutation. Sectors 1 and 2 form a dual-slot journal: boot validates both records,
selects the highest valid committed sequence, then appends the next sequence to
the alternate slot and verifies it by readback without destroying the prior
commit. The VM gate creates sequences 1 and 2, corrupts the latest slot, and
requires fallback to sequence 1 before sequence 2 is reconstructed. This is a
bounded two-record rollback window, not yet a general filesystem or
kotobase IStore. The blk queue uses MSI-X vector 35 for every synchronous
sector completion and sleeps with interrupts enabled instead of polling its
used ring. Both blk and rng perform a bounded MSI-X capability walk, validate
the complete table and PBA against independently probed BAR extents, map their
MMIO UC/NX, and require real IRQ evidence before accepting DMA completion
(vector 34 for rng and vector 35 for blk). MSI-X for the remaining transports,
IOMMU isolation, indirect descriptors, and a reusable multi-request transport
remain later Phase 4 work.

ACPI DMAR discovery validates the complete table, bounded remapping structures,
DRHD register bases, and variable-length device scopes. A detected but
unconfigured VT-d unit makes PCI DMA fail closed. Only the QEMU bring-up profile
may use unisolated DMA when DMAR is absent, and its serial evidence explicitly
labels that exception `test-only-unisolated` rather than claiming isolation.

The desktop transport bootstrap obtains the active UEFI GOP mode before
`ExitBootServices` and hands only the aperture base/length, dimensions, stride,
and RGB/BGR format to the kernel. The kernel independently validates every
bound, maps the aperture supervisor-only RW+NX and uncached in a dedicated page
directory, then presents a deterministic retained-rectangle test frame. A
stable readback hash is required before `AIUEOS_FRAMEBUFFER_OK` is emitted.
This is the native display capability boundary for the browser-owned desktop:
the browser remains the workspace/focus/permission authority, while the kernel
only admits validated surfaces and hardware input. Direct framebuffer mapping
is not granted to the browser. The input boundary uses a versioned, sequenced
envelope (`pointer`, `key`, or `text`); raw virtio DMA memory stays kernel-only
and IME interpretation belongs to the browser desktop authority. The QEMU
smoke configures a real modern `virtio-keyboard-pci` event queue, but its event
is explicitly synthetic because headless HMP `sendkey` is routed to the legacy
console rather than virtio-keyboard. Production builds do not enable that
fallback and require a device-completed, length/type/value-validated event.
into a user component, ambient display authority, and an invented browser
runtime are intentionally excluded.

```sh
./os/aiueos/scripts/build-uefi.sh
./os/aiueos/scripts/smoke-qemu-uefi.sh
./os/aiueos/scripts/build-release-image.sh
./os/aiueos/scripts/smoke-qemu-release-image.sh
```

The release-image command creates a deterministic 64 MiB GPT raw disk image
with a protective MBR and a FAT32 EFI System Partition. The ESP contains
`EFI/BOOT/BOOTX64.EFI` and `EFI/AIUEOS/KERNEL.ELF`. It also emits a canonical
JSON build receipt with the SHA-256 digest and byte size of the disk and both
boot artifacts. Set `SOURCE_DATE_EPOCH` to record a release timestamp without
making the disk image host-time-dependent. The image builder uses only Python's
standard library; validation checks both GPT CRCs, the ESP layout, FAT chains,
boot-image magic, and byte-for-byte embedded artifact contents.

Requirements are Zig 0.14 or newer and `qemu-system-x86_64` with an edk2/OVMF
firmware image. Override firmware discovery with `OVMF_CODE=/path/to/code.fd`.

The EFI application is deliberately a small native bootstrap substrate. Kotoba
programs use the freestanding target contract in `kotoba-lang/compiler`; moving
the remaining bootstrap into compiler-emitted PE/COFF is tracked by the ADR.
