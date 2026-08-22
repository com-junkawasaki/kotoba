#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
aiueos="$repo/os/aiueos"
out=${AIUEOS_OUT:-"$repo/build/aiueos"}
log="$out/uefi-debug.log"
serial_log="$out/kernel-serial.log"
blk_image="$out/virtio-blk-smoke.img"
qemu=${QEMU_SYSTEM_X86_64:-qemu-system-x86_64}

AIUEOS_INPUT_SMOKE_SYNTHETIC=1 "$aiueos/scripts/build-uefi.sh" >/dev/null
if [ "${AIUEOS_CORRUPT_KERNEL:-0}" = 1 ]; then
  python3 - "$out/esp/EFI/AIUEOS/KERNEL.ELF" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
data = bytearray(path.read_bytes())
data[-1] ^= 0x01
path.write_bytes(data)
PY
fi
command -v "$qemu" >/dev/null 2>&1 || {
  echo "error: qemu-system-x86_64 is required" >&2
  exit 1
}

if [ -z "${OVMF_CODE:-}" ]; then
  for candidate in \
    /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
    /opt/homebrew/Cellar/qemu/*/share/qemu/edk2-x86_64-code.fd \
    /usr/share/OVMF/OVMF_CODE_4M.fd \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/edk2/x64/OVMF_CODE.fd; do
    if [ -f "$candidate" ]; then OVMF_CODE=$candidate; break; fi
  done
fi
[ -f "${OVMF_CODE:-}" ] || {
  echo "error: OVMF firmware not found; set OVMF_CODE" >&2
  exit 1
}

rm -f "$log" "$serial_log"
if [ "${AIUEOS_PRESERVE_BLK_IMAGE:-0}" != 1 ] || [ ! -f "$blk_image" ]; then
python3 - "$blk_image" <<'PY'
import pathlib
import struct
import sys

path = pathlib.Path(sys.argv[1])
payload = bytearray(1024 * 1024)
obj = b"KOTOBASE-ROOT-V1"
checksum = 2166136261
for byte in obj:
    checksum = ((checksum ^ byte) * 16777619) & 0xffffffff
header = struct.pack("<8s7I", b"AIUEFS1\0", 1, 36, 1, 0, 64, len(obj), checksum)
payload[:len(header)] = header
payload[64:64 + len(obj)] = obj
path.write_bytes(payload)
PY
fi
if [ -n "${AIUEOS_DISK_IMAGE:-}" ]; then
  [ -f "$AIUEOS_DISK_IMAGE" ] || {
    echo "error: AIUEOS_DISK_IMAGE does not exist: $AIUEOS_DISK_IMAGE" >&2
    exit 1
  }
  # OVMF may open the boot medium writable; snapshot keeps the release artifact immutable.
  boot_drive="format=raw,snapshot=on,file=$AIUEOS_DISK_IMAGE"
else
  boot_drive="format=raw,file=fat:rw:$out/esp"
fi
set +e
iommu_args=
if [ "${AIUEOS_TEST_DMAR:-0}" = 1 ]; then iommu_args="-device intel-iommu"; fi
# shellcheck disable=SC2086 # intentional optional pair of QEMU arguments
"$qemu" \
  -machine q35,accel=tcg -cpu max -m 128M -smp 2 \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive "$boot_drive" \
  -device isa-debugcon,iobase=0xe9,chardev=debug \
  -chardev file,id=debug,path="$log" \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
  $iommu_args \
  -device virtio-rng-pci \
  -drive if=none,id=aiueosblk,format=raw,file="$blk_image" \
  -device virtio-blk-pci,drive=aiueosblk,disable-legacy=on \
  -device virtio-keyboard-pci,disable-legacy=on \
  -display none -serial "file:$serial_log" -monitor none -no-reboot
status=$?
set -e

if [ "${AIUEOS_CORRUPT_KERNEL:-0}" = 1 ]; then
  [ "$status" -eq 255 ] || {
    echo "error: corrupted kernel produced unexpected QEMU status $status" >&2
    exit 1
  }
  grep -F "AIUEOS_LOADER_FAIL kernel-sha256" "$log" >/dev/null || {
    echo "error: corrupted kernel was not rejected by loader" >&2
    exit 1
  }
  echo "AIUEOS_KERNEL_INTEGRITY_REJECTION_OK"
  exit 0
fi

if [ "${AIUEOS_TEST_DMAR:-0}" = 1 ]; then
  # PCI DMA is intentionally rejected before a virtqueue can be submitted.
  [ "$status" -eq 233 ] || {
    echo "error: fail-closed DMAR profile exited with status $status" >&2; exit 1;
  }
  grep -F "AIUEOS_DMA_POLICY_OK dmar=validated dma=denied-until-vtd-enabled" "$serial_log" >/dev/null || {
    echo "error: validated DMAR fail-closed evidence was not observed" >&2; exit 1;
  }
  echo "AIUEOS_DMAR_FAIL_CLOSED_OK"
  exit 0
fi

# The #UD handler writes 0x30; isa-debug-exit maps it to (0x30 << 1) | 1 = 97.
[ "$status" -eq 97 ] || {
  echo "error: unexpected QEMU exit status $status" >&2
  test -f "$log" && sed -n '1,80p' "$log" >&2
  exit 1
}
grep -F "AIUEOS_LOADER_OK" "$log" >/dev/null || {
  echo "error: loader identity was not observed" >&2
  exit 1
}
grep -F "AIUEOS_GOP_HANDOFF_OK framebuffer-v1" "$log" >/dev/null || {
  echo "error: loader did not hand off a validated GOP mode" >&2
  exit 1
}
grep -F "AIUEOS_LOADER_INTEGRITY_OK sha256-v1" "$log" >/dev/null || {
  echo "error: kernel integrity evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_KERNEL_OK memory-map-v1" "$log" >/dev/null || {
  echo "error: kernel handoff was not observed" >&2
  exit 1
}
grep -F "AIUEOS_SERIAL_OK stack-v1 memory-map-v1" "$serial_log" >/dev/null || {
  echo "error: kernel COM1 evidence was not observed" >&2
  test -f "$serial_log" && sed -n '1,80p' "$serial_log" >&2
  exit 1
}
grep -F "AIUEOS_DESCRIPTOR_TABLES_OK gdt-v1 idt-v1" "$serial_log" >/dev/null || {
  echo "error: kernel descriptor-table evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_PAGING_OK cr3-owned wx-v1 nx-wp" "$serial_log" >/dev/null || {
  echo "error: kernel-owned paging evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_FRAMEBUFFER_OK gop-owned retained-rectangles hash-verified" "$serial_log" >/dev/null || {
  echo "error: kernel did not validate and render the GOP framebuffer" >&2
  exit 1
}
grep -F "AIUEOS_PHYSICAL_ALLOCATOR_OK pages=2 zeroed" "$serial_log" >/dev/null || {
  echo "error: physical page allocator evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_ACPI_OK rsdp-xsdt-madt cpu>=2" "$serial_log" >/dev/null || {
  echo "error: validated ACPI CPU discovery evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_DMA_POLICY_OK dmar=absent test-only-unisolated" "$serial_log" >/dev/null || {
  echo "error: explicit no-IOMMU test DMA policy evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_APIC_TIMER_OK vector=32 eoi-v1" "$serial_log" >/dev/null || {
  echo "error: Local APIC timer interrupt evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_SMP_OK cpus=2 init-sipi-v1 per-cpu-stack" "$serial_log" >/dev/null || {
  echo "error: BSP-to-AP INIT/SIPI evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_PCI_OK bounded-scan virtio-vendor=1af4" "$serial_log" >/dev/null || {
  echo "error: bounded PCI/virtio discovery evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_VIRTIO_RNG_OK modern-pci caps-bounded dma=4pages completion=32" "$serial_log" >/dev/null || {
  echo "error: modern virtio-rng DMA completion evidence was not observed" >&2
  test -f "$serial_log" && sed -n '1,120p' "$serial_log" >&2
  exit 1
}
grep -F "AIUEOS_VIRTIO_RNG_MSIX_OK vector=34 irq=1 table-pba-bounded" "$serial_log" >/dev/null || {
  echo "error: interrupt-driven virtio-rng MSI-X evidence was not observed" >&2
  test -f "$serial_log" && sed -n '1,140p' "$serial_log" >&2
  exit 1
}
grep -F "AIUEOS_VIRTIO_BLK_OK capacity-bounded sector=0 bytes=512 readonly" "$serial_log" >/dev/null || {
  echo "error: modern virtio-blk bounded read evidence was not observed" >&2
  test -f "$serial_log" && sed -n '1,140p' "$serial_log" >&2
  exit 1
}
grep -F "AIUEOS_VIRTIO_BLK_MSIX_OK vector=35 irq-completions-bounded table-pba-bounded" "$serial_log" >/dev/null || {
  echo "error: interrupt-driven virtio-blk MSI-X completion evidence was not observed" >&2
  test -f "$serial_log" && sed -n '1,160p' "$serial_log" >&2
  exit 1
}
grep -F "AIUEOS_OBJECT_STORE_OK aiuefs-v1 objects=1 checksum=fnv1a" "$serial_log" >/dev/null || {
  echo "error: bounded read-only object-store evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_JOURNAL_OK dual-slot committed append-readback" "$serial_log" >/dev/null || {
  echo "error: journal write/readback evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_VIRTIO_INPUT_OK modern-pci eventq configured synthetic-smoke" "$serial_log" >/dev/null || {
  echo "error: modern virtio-input configuration/synthetic transport evidence was not observed" >&2; exit 1;
}
grep -F "AIUEOS_DESKTOP_INPUT_OK envelope-v1 sequence=1 kind=key ime-neutral" "$serial_log" >/dev/null || {
  echo "error: validated browser desktop input envelope was not observed" >&2; exit 1;
}
grep -F "AIUEOS_SCHEDULER_OK tasks=2 policy=round-robin preemption=apic-timer" "$serial_log" >/dev/null || {
  echo "error: preemptive round-robin scheduler evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_SCHEDULER_CR3_OK roots=3 private-pages=2 kernel-return" "$serial_log" >/dev/null || {
  echo "error: scheduler-driven address-space switching evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_IOAPIC_OK pit-gsi vector=33 eoi-v1" "$serial_log" >/dev/null || {
  echo "error: IOAPIC external timer IRQ evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_SYSCALL_OK int80-cpl0 abi-v1" "$serial_log" >/dev/null || {
  echo "error: CPL0 syscall evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_CAPABILITY_OK handle-v1 invalid-handle-denied" "$serial_log" >/dev/null || {
  echo "error: capability negative evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_PROCESS_FOUNDATION_OK tss-descriptor user-wx guard-page" "$serial_log" >/dev/null || {
  echo "error: process isolation foundation evidence was not observed" >&2; exit 1;
}
grep -F "AIUEOS_ADDRESS_SPACE_OK processes=2 distinct-cr3 private-pages cross-access-fault" "$serial_log" >/dev/null || {
  echo "error: per-process address-space isolation evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_RING3_OK cpl3-int80 tss-rsp0 return-kernel" "$serial_log" >/dev/null || {
  echo "error: CPL3 syscall and kernel-return evidence was not observed" >&2; exit 1;
}
grep -F "AIUEOS_USER_SYSCALL_OK valid-log invalid-handle invalid-pointer" "$serial_log" >/dev/null || {
  echo "error: CPL3 syscall positive/negative evidence was not observed" >&2; exit 1;
}
grep -F "AIUEOS_COPYIN_OK noncanonical-and-unmapped-denied" "$serial_log" >/dev/null || {
  echo "error: invalid-pointer evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_PAGE_FAULT_OK write-protect vector=14" "$serial_log" >/dev/null || {
  echo "error: write-protect page-fault evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_PAGE_FAULT_OK no-execute vector=14" "$serial_log" >/dev/null || {
  echo "error: no-execute page-fault evidence was not observed" >&2
  exit 1
}
grep -F "AIUEOS_EXCEPTION_OK vector=6 invalid-opcode" "$serial_log" >/dev/null || {
  echo "error: kernel exception dispatch evidence was not observed" >&2
  exit 1
}
echo "AIUEOS_UEFI_SMOKE_OK"
