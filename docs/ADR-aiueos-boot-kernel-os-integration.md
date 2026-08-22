# ADR — aiueos boot, kernel, image, and OS integration

- **Status**: Accepted; implementation staged
- **Date**: 2026-07-14
- **Owners**: `kotoba-lang/kotoba` (product integration),
  `kotoba-lang/compiler` (code generation), `kotoba-lang/aiueos`
  (capability/component semantics)
- **Related implementation**: `kotoba-lang/aiueos#29`, the reviewed replacement
  for PR #25

## Context

aiueos currently names both a capability-secure component contract and an
intended machine OS. These are different maturity surfaces. PR #29 restores a
Linux-hosted initramfs/PID-1/QEMU path, portable virtio-blk/console logic, and
an experimental JVM FFM/VFIO provider. It is not a bare-metal kernel: Linux
still owns firmware handoff, PCI/IOMMU, interrupts, paging, scheduling, and
virtual memory. The VFIO provider is also not yet wired into aiueos's Wasm
host-import quartet; those imports remain deterministic stubs.

## Decision

### Ownership

`kotoba-lang/kotoba` owns the composition of the bootable product:

- boot profiles and release graph;
- firmware-to-kernel, kernel, syscall, and driver ABIs;
- ISO/raw-disk/initramfs composition;
- QEMU and real-machine evidence;
- aiueos, Kotoba, kototama, browser, and kotobase integration.

`kotoba-lang/compiler` owns genuinely freestanding targets:

- `x86_64-aiueos-kernel`, then `aarch64-aiueos-kernel`;
- `x86_64-aiueos-uefi`;
- PE/COFF and ELF emission;
- relocation, sections, entry point, stack, TLS, and no-host-runtime contracts.

`kotoba-lang/aiueos` remains the authority for manifests, policy, admission,
audit/run receipts, component boot graphs, portable virtio protocol logic, and
the Linux-hosted development profile.

### Profiles

`hosted-linux` is Phase 0:

```text
firmware -> Linux -> initramfs -> JVM/aiueos PID 1
                              -> Chicory/Kotoba components
                              -> optional Linux VFIO provider
```

`bare-metal` is Phases 1–6:

```text
UEFI or BIOS/GRUB
  -> aiueos loader and native kernel
  -> ACPI/SMP/paging/APIC/IOMMU
  -> scheduler/virtual memory/syscalls
  -> PCI/MMIO/DMA/IRQ drivers
  -> kototama/Kotoba components
  -> browser shell + kotobase persistence
```

No hosted result satisfies a bare-metal release gate.

### Required artifacts

| Artifact | Purpose |
|---|---|
| `BOOTX64.EFI` | primary UEFI loader |
| BIOS stage-1 sector | legacy boot test fixture |
| GRUB/Multiboot2 configuration | compatibility boot path |
| PE/COFF loader | UEFI-loadable compiler output |
| bootable ISO | VM/distribution image |
| GPT raw disk image | USB/QEMU/real-machine boot |
| kernel image | native aiueos kernel |
| `newc` initramfs/cpio | early components and recovery |

Each artifact is reproducible, hashed, signed, and accompanied by a build
receipt. A file-shaped placeholder does not count; successful QEMU boot is the
minimum evidence.

### Kernel scope

The native kernel must implement:

- firmware memory-map ingestion;
- ACPI RSDP/XSDT/MADT and CPU discovery;
- SMP application-processor startup;
- page tables, W^X, isolation, and guard pages;
- physical/virtual memory allocators;
- APIC, timer, exceptions, and interrupt dispatch;
- preemptive scheduler and address spaces;
- capability-handle tables;
- syscall entry/exit, validation, and copy-in/copy-out;
- PCI enumeration and BAR validation;
- MMIO, DMA, IOMMU, and IRQ providers;
- serial console, panic/crash receipt, and deterministic QEMU shutdown.

The first syscall ABI is capability-handle based. POSIX is an optional service,
not the kernel authority.

### Compiler and native-substrate rule

Policy, service, driver-protocol, and application code expressible in Kotoba is
compiled by `kotoba-lang/compiler`. A small assembly/native substrate is
allowed temporarily for reset entry, CPU mode transition, page-table
activation, interrupt stubs, and context switch.

Every exception requires a named ABI, QEMU positive/negative tests, a compiler
migration issue, and no ambient authority above the capability boundary.
Hosted KEXE targets are not kernel targets. A target is freestanding only when
it has no supervisor, libc, JVM, or host syscall dependency.

### Driver, UI, and persistence split

```text
portable virtio planner
  -> admitted driver service
  -> kernel queue/MMIO/DMA/IRQ provider
  -> device
```

VFIO remains a hosted conformance provider. Bare metal owns PCI, IOMMU, and
interrupt setup.

`kotoba-lang/browser` supplies shell/window/workspace state, input vocabulary,
and retained draw operations. The native release additionally requires
framebuffer/virtio-gpu scanout, compositor, virtio-input/USB HID, keyboard/IME,
accessibility, and clipboard/file-picker permission brokers.

`kotobase` is the datom persistence plane, not a block driver:

```text
virtio-blk/NVMe -> block service -> filesystem/object store
                -> kotobase IStore -> browser/profile/system datoms
```

### Phases and exit gates

| Phase | Deliverable | Exit gate |
|---|---|---|
| 0 | Linux PID-1/initramfs/QEMU/virtio/VFIO prototype | PR #29 merged; unit CI green; whole boot still unproven |
| 1 | UEFI loader + serial kernel | In progress: OVMF hands off to a bounded ELF64 kernel with its own stack and COM1 serial output; signature verification remains |
| 2 | paging, exceptions, ACPI, APIC, SMP | In progress: GDT/IDT, CR3/W^X, ACPI discovery, and BSP Local APIC timer vector 32 pass; IOAPIC and AP startup remain |
| 3 | scheduler, VM, syscall, capability handles | isolated tasks; W^X and invalid-handle tests |
| 4 | PCI/MMIO/DMA/IOMMU/IRQ + virtio | real QEMU queue completion; malformed descriptors rejected |
| 5 | ISO/GPT/raw image, recovery, signed update | reproducible UEFI and GRUB boots |
| 6 | browser shell, compositor, input, kotobase | desktop session persists and restores state |

Contract M6 does not imply kernel/hardware M6. Every subsystem records maturity
separately.

## Security invariants

- Firmware tables, descriptors, disk/package data, and binaries are hostile.
- DMA is disabled until IOMMU isolation, except in a named QEMU-only profile.
- Components receive revocable bounded handles, never physical addresses.
- Executable mappings are not writable after admission.
- Loader, kernel, policy, components, and filesystem identities are bound into
  one boot receipt.
- Secure Boot is not claimed until PE/COFF signing and key lifecycle are tested
  on real firmware.

## Consequences

- aiueos PR #29 is supported but classified as Linux-hosted Phase 0.
- The bootable product has one integration owner.
- The compiler must gain freestanding targets before claiming a Kotoba-compiled
  kernel.
- Native bootstrap code is constrained and auditable rather than hidden.
- Parity with macOS, Windows, or Linux is not claimed until Phase 6 passes on a
  real-machine class as well as QEMU.

## Implementation record

The Phase 1 vertical slice lives in `os/aiueos`. It builds a real PE32+ EFI
application and a separate static ELF64 kernel with a freestanding toolchain.
The loader reads the kernel from the ESP, validates bounded load segments and
its executable entry, captures the UEFI memory map, exits boot services, and
hands a versioned boot-info structure to the kernel. The kernel validates that
handoff, emits `AIUEOS_KERNEL_OK`, and uses a test-only I/O device for
deterministic shutdown. The assembly entry replaces the firmware stack with a
private 64 KiB stack, while the kernel initializes COM1 and emits independent
serial evidence. The guest contains no Linux, libc, JVM, initramfs, or hosted
supervisor.

This evidence proves firmware entry, PE/COFF packaging, a separate kernel
image, memory-map handoff, post-boot-services kernel execution, an owned kernel
stack, and COM1 output. It does not yet prove signature verification or any
complete Phase 2 kernel mechanism. The first Phase 2 slice replaces the
firmware GDT/IDT and proves exception dispatch by executing `ud2` and observing
the kernel's vector 6 handler; other exception stubs, paging, ACPI, APIC, and
SMP remain. The next slice installs a kernel-owned four-level bootstrap map,
sets CR0.WP and EFER.NXE, and separates text (RX), rodata (R+NX), and mutable
state (RW+NX). Vector 14 recovery verifies both a forbidden text write and a
forbidden instruction fetch from rodata, including the x86 error-code bits,
before the vector 6 regression probe runs.

The loader passes only the ACPI 2.0 RSDP selected by its UEFI GUID. The kernel
validates legacy and extended RSDP checksums, then applies bounded signature,
length, checksum, and subtable-walk checks to XSDT and MADT. The two-vCPU QEMU
gate requires at least two enabled Local APIC/x2APIC processor records. This is
discovery evidence, not SMP startup evidence.

The BSP Local APIC slice maps the xAPIC MMIO window cache-disabled, enables the
spurious vector, and programs a periodic timer on vector 32. The QEMU gate must
wake from `sti; hlt`, enter the kernel interrupt stub, issue EOI, and continue.
It does not yet route external interrupts or start application processors.

The external-interrupt slice retains the MADT IOAPIC and IRQ0 source override,
maps IOAPIC MMIO UC/NX, masks both legacy PICs, and routes the PIT through GSI
to vector 33. The smoke gate must wake through that external IRQ and issue a
Local APIC EOI. MSI/MSI-X routing remains separate.

The first physical allocator consumes the variable-stride UEFI memory map and
admits only Conventional Memory above the kernel image and below the current
1 GiB bootstrap identity limit. Allocations are page-aligned and zeroed. This
bounded bump allocator establishes ownership evidence; reclamation, free lists,
zones, and allocation above the bootstrap map remain.

The Phase 4 discovery slice performs a bounded PCI configuration mechanism #1
scan and requires a QEMU virtio function with vendor ID `0x1af4`. It does not
yet admit BARs, map device MMIO, allocate DMA, route MSI/MSI-X, or operate a
virtqueue.

The first storage slice uses a separately attached writable virtio-blk fixture,
isolated from the ESP and release disk. Sector 0 is an `aiuefs-v1` object-store
superblock: header size, object count, offset, length, and checksum are bounded
within one sector before an object is admitted. Sectors 1 and 2 are bounded
journal slots. Before mutation, the kernel validates both slots' magic, version,
nonzero sequence, committed state, payload bounds, and independent header and
payload checksums. It selects the highest valid sequence and appends its
successor to the alternate slot, preserving the selected commit until the new
record has been written and read back. CI creates sequences 1 and 2, corrupts
the latest slot, then requires fallback to sequence 1 and reconstruction of
sequence 2. This closes a one-commit rollback window, but is not yet an
unbounded multi-record journal, writable object allocator, filesystem, or
kotobase IStore.

The block request queue is assigned MSI-X table entry 0 and architectural
vector 35 before `DRIVER_OK`. The kernel bounds the advertised vector count,
table and PBA byte ranges against probed BAR extents, maps only those validated
MMIO ranges, and rejects a queue vector assignment that the device does not
retain. Every sector operation waits in `sti; hlt` and is accepted only when
both a vector-35 IRQ and the matching used-ring element are observed. This is
real QEMU device-interrupt evidence; it is not a synthetic smoke event.

## Initial non-goals

- full POSIX/Linux ABI compatibility;
- every x86 chipset or GPU;
- BIOS as the primary production path;
- Windows/macOS binary compatibility;
- safety certification or hard real-time guarantees.
