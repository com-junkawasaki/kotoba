#include <stdint.h>

struct aiueos_boot_info {
  uint64_t magic, version;
  void *memory_map; uint64_t memory_map_size, descriptor_size, descriptor_version;
  void *acpi_rsdp;
  uint64_t framebuffer_base, framebuffer_size;
  uint32_t framebuffer_width, framebuffer_height, framebuffer_stride, framebuffer_format;
};

struct __attribute__((packed)) idt_entry {
  uint16_t offset_low, selector;
  uint8_t ist, attributes;
  uint16_t offset_middle;
  uint32_t offset_high, reserved;
};
struct __attribute__((packed)) descriptor_pointer {
  uint16_t limit;
  uint64_t base;
};

extern void aiueos_load_gdt(void);
extern void aiueos_load_idt(const struct descriptor_pointer *pointer);
extern void aiueos_isr_invalid_opcode(void);
extern void aiueos_isr_page_fault(void);
extern void aiueos_isr_apic_timer(void);
extern void aiueos_isr_external_timer(void);
extern void aiueos_isr_virtio_rng(void);
extern void aiueos_isr_virtio_blk(void);
extern void aiueos_isr_syscall(void);
extern void aiueos_probe_write_protect(void);
extern void aiueos_probe_no_execute(void);
volatile uint64_t aiueos_page_fault_stage;
volatile uint64_t aiueos_page_fault_error;
extern int aiueos_paging_initialize(void);
extern int aiueos_framebuffer_initialize(const struct aiueos_boot_info *boot);
extern int aiueos_acpi_initialize(const void *rsdp);
extern int aiueos_dma_test_policy_allows_unisolated(void);
extern int aiueos_apic_timer_initialize(void);
extern volatile uint64_t aiueos_apic_timer_ticks;
extern int aiueos_physical_allocator_initialize(const struct aiueos_boot_info *boot);
extern void *aiueos_allocate_physical_page(void);
extern int aiueos_pci_enumerate(void);
extern int aiueos_object_store_ready(void);
extern int aiueos_journal_ready(void);
extern int aiueos_journal_recovered(void);
extern uint32_t aiueos_journal_sequence(void);
extern uint32_t aiueos_journal_recovered_sequence(void);
extern uint32_t aiueos_journal_slot(void);
extern void aiueos_scheduler_initialize(void);
extern int aiueos_scheduler_evidence_ready(void);
extern int aiueos_syscall_self_test(void);
extern int aiueos_process_initialize(void);
extern void aiueos_process_enter(void);
extern int aiueos_process_result(void);
extern int aiueos_address_space_self_test(void);
extern void aiueos_load_task_register(void);
extern int aiueos_smp_start_application_processor(void);
extern int aiueos_ioapic_route_legacy_timer(void);
extern volatile uint64_t aiueos_external_timer_ticks;
extern volatile uint64_t aiueos_virtio_rng_irq_count;
extern volatile uint64_t aiueos_virtio_blk_irq_count;
static struct idt_entry idt[256] __attribute__((aligned(16)));

static inline void debug_byte(uint8_t value) {
  __asm__ volatile("outb %0, $0xe9" : : "a"(value));
}
static inline void out8(uint16_t port, uint8_t value) {
  __asm__ volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}
static inline uint8_t in8(uint16_t port) {
  uint8_t value;
  __asm__ volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
  return value;
}
static void serial_init(void) {
  out8(0x3f8 + 1, 0x00);
  out8(0x3f8 + 3, 0x80);
  out8(0x3f8 + 0, 0x01);
  out8(0x3f8 + 1, 0x00);
  out8(0x3f8 + 3, 0x03);
  out8(0x3f8 + 2, 0xc7);
  out8(0x3f8 + 4, 0x0b);
}
static void serial_byte(uint8_t value) {
  uint32_t budget = 1000000;
  while (!(in8(0x3f8 + 5) & 0x20) && --budget) {}
  if (budget) out8(0x3f8, value);
}
static void serial_string(const char *text) {
  while (*text) serial_byte((uint8_t)*text++);
}
static void debug_string(const char *text) {
  while (*text) debug_byte((uint8_t)*text++);
}
static inline void qemu_exit(uint32_t value) {
  __asm__ volatile("outl %0, $0xf4" : : "a"(value));
}

static void set_idt_gate(uint8_t vector, void (*handler)(void)) {
  uint64_t address = (uint64_t)(uintptr_t)handler;
  idt[vector].offset_low = (uint16_t)address;
  idt[vector].selector = 0x08;
  idt[vector].ist = 0;
  idt[vector].attributes = 0x8e;
  idt[vector].offset_middle = (uint16_t)(address >> 16);
  idt[vector].offset_high = (uint32_t)(address >> 32);
  idt[vector].reserved = 0;
}

__attribute__((noreturn))
void aiueos_exception_dispatch(uint64_t vector) {
  if (vector == 6) {
    debug_string("AIUEOS_EXCEPTION_OK vector=6\n");
    serial_string("AIUEOS_EXCEPTION_OK vector=6 invalid-opcode\r\n");
    qemu_exit(0x30);
  } else {
    debug_string("AIUEOS_EXCEPTION_FAIL unexpected-vector\n");
    serial_string("AIUEOS_EXCEPTION_FAIL unexpected-vector\r\n");
    qemu_exit(0x7d);
  }
  for (;;) __asm__ volatile("cli; hlt");
}

__attribute__((noreturn))
void aiueos_kernel_main(const struct aiueos_boot_info *boot) {
  serial_init();
  if (!boot || boot->magic != 0x414955454f53424fULL || boot->version != 1 ||
      !boot->memory_map || !boot->memory_map_size || !boot->descriptor_size) {
    debug_string("AIUEOS_KERNEL_FAIL boot-info\n");
    serial_string("AIUEOS_KERNEL_FAIL boot-info\r\n");
    qemu_exit(0x7e);
  } else {
    debug_string("AIUEOS_KERNEL_OK memory-map-v1\n");
    serial_string("AIUEOS_SERIAL_OK stack-v1 memory-map-v1\r\n");
    aiueos_load_gdt();
    set_idt_gate(6, aiueos_isr_invalid_opcode);
    set_idt_gate(14, aiueos_isr_page_fault);
    set_idt_gate(32, aiueos_isr_apic_timer);
    set_idt_gate(33, aiueos_isr_external_timer);
    set_idt_gate(34, aiueos_isr_virtio_rng);
    set_idt_gate(35, aiueos_isr_virtio_blk);
    set_idt_gate(128, aiueos_isr_syscall);
    idt[128].attributes = 0xee; /* present, DPL3, interrupt gate */
    const struct descriptor_pointer idtr = {
      .limit = (uint16_t)(sizeof(idt) - 1),
      .base = (uint64_t)(uintptr_t)idt
    };
    aiueos_load_idt(&idtr);
    debug_string("AIUEOS_DESCRIPTOR_TABLES_OK gdt-v1 idt-v1\n");
    serial_string("AIUEOS_DESCRIPTOR_TABLES_OK gdt-v1 idt-v1\r\n");
    if (!aiueos_paging_initialize()) {
      debug_string("AIUEOS_PAGING_FAIL ownership-or-wx\n");
      serial_string("AIUEOS_PAGING_FAIL ownership-or-wx\r\n");
      qemu_exit(0x7c);
    }
    debug_string("AIUEOS_PAGING_OK cr3-owned wx-v1 nx-wp\n");
    serial_string("AIUEOS_PAGING_OK cr3-owned wx-v1 nx-wp\r\n");
    if (!aiueos_framebuffer_initialize(boot)) {
      debug_string("AIUEOS_FRAMEBUFFER_FAIL gop-contract\n");
      serial_string("AIUEOS_FRAMEBUFFER_FAIL gop-contract\r\n");
      qemu_exit(0x68);
    }
    debug_string("AIUEOS_FRAMEBUFFER_OK gop-owned retained-rectangles hash-verified\n");
    serial_string("AIUEOS_FRAMEBUFFER_OK gop-owned retained-rectangles hash-verified\r\n");
    if (!aiueos_physical_allocator_initialize(boot)) {
      debug_string("AIUEOS_PHYSICAL_ALLOCATOR_FAIL memory-map\n");
      serial_string("AIUEOS_PHYSICAL_ALLOCATOR_FAIL memory-map\r\n");
      qemu_exit(0x76);
    }
    void *physical_page_a = aiueos_allocate_physical_page();
    void *physical_page_b = aiueos_allocate_physical_page();
    if (!physical_page_a || !physical_page_b || physical_page_a == physical_page_b ||
        ((uintptr_t)physical_page_a & 4095) || ((uintptr_t)physical_page_b & 4095) ||
        *(const uint64_t *)physical_page_a || *(const uint64_t *)physical_page_b) {
      debug_string("AIUEOS_PHYSICAL_ALLOCATOR_FAIL allocation\n");
      serial_string("AIUEOS_PHYSICAL_ALLOCATOR_FAIL allocation\r\n");
      qemu_exit(0x75);
    }
    debug_string("AIUEOS_PHYSICAL_ALLOCATOR_OK pages=2 zeroed\n");
    serial_string("AIUEOS_PHYSICAL_ALLOCATOR_OK pages=2 zeroed\r\n");
    if (!aiueos_acpi_initialize(boot->acpi_rsdp)) {
      debug_string("AIUEOS_ACPI_FAIL rsdp-xsdt-madt\n");
      serial_string("AIUEOS_ACPI_FAIL rsdp-xsdt-madt\r\n");
      qemu_exit(0x78);
    }
    debug_string("AIUEOS_ACPI_OK rsdp-xsdt-madt cpu>=2\n");
    serial_string("AIUEOS_ACPI_OK rsdp-xsdt-madt cpu>=2\r\n");
    if (!aiueos_dma_test_policy_allows_unisolated()) {
      serial_string("AIUEOS_DMA_POLICY_OK dmar=validated dma=denied-until-vtd-enabled\r\n");
    } else {
      serial_string("AIUEOS_DMA_POLICY_OK dmar=absent test-only-unisolated\r\n");
    }
    if (!aiueos_apic_timer_initialize()) {
      debug_string("AIUEOS_APIC_FAIL initialization\n");
      serial_string("AIUEOS_APIC_FAIL initialization\r\n");
      qemu_exit(0x77);
    }
    if (!aiueos_smp_start_application_processor()) {
      debug_string("AIUEOS_SMP_FAIL ap-startup\n");
      serial_string("AIUEOS_SMP_FAIL ap-startup\r\n");
      qemu_exit(0x73);
    }
    debug_string("AIUEOS_SMP_OK cpus=2 init-sipi-v1\n");
    serial_string("AIUEOS_SMP_OK cpus=2 init-sipi-v1 per-cpu-stack\r\n");
    aiueos_scheduler_initialize();
    __asm__ volatile("sti");
    while (!aiueos_scheduler_evidence_ready()) __asm__ volatile("hlt");
    __asm__ volatile("cli");
    debug_string("AIUEOS_APIC_TIMER_OK vector=32 eoi-v1\n");
    serial_string("AIUEOS_APIC_TIMER_OK vector=32 eoi-v1\r\n");
    int pci_result = aiueos_pci_enumerate();
    if (!pci_result) {
      debug_string("AIUEOS_PCI_FAIL enumeration-or-virtio\n");
      serial_string("AIUEOS_PCI_FAIL enumeration-or-virtio\r\n");
      qemu_exit(0x74);
    }
    debug_string("AIUEOS_PCI_OK bounded-scan virtio-vendor=1af4\n");
    serial_string("AIUEOS_PCI_OK bounded-scan virtio-vendor=1af4\r\n");
    if (pci_result < 2) {
      debug_string("AIUEOS_VIRTIO_FAIL rng-queue\n");
      serial_string("AIUEOS_VIRTIO_FAIL rng-queue\r\n");
      qemu_exit(0x73);
    }
    debug_string("AIUEOS_VIRTIO_RNG_OK modern-pci caps-bounded dma=4pages completion=32\n");
    serial_string("AIUEOS_VIRTIO_RNG_OK modern-pci caps-bounded dma=4pages completion=32\r\n");
    if (aiueos_virtio_rng_irq_count != 1) {
      serial_string("AIUEOS_VIRTIO_RNG_MSIX_FAIL irq-count\r\n"); qemu_exit(0x6f);
    }
    debug_string("AIUEOS_VIRTIO_RNG_MSIX_OK vector=34 irq=1 table-pba-bounded\n");
    serial_string("AIUEOS_VIRTIO_RNG_MSIX_OK vector=34 irq=1 table-pba-bounded\r\n");
    if ((pci_result & 3) != 3) {
      debug_string("AIUEOS_VIRTIO_BLK_FAIL capacity-or-read\n");
      serial_string("AIUEOS_VIRTIO_BLK_FAIL capacity-or-read\r\n");
      qemu_exit(0x71);
    }
    debug_string("AIUEOS_VIRTIO_BLK_OK capacity-bounded sector=0 bytes=512 readonly\n");
    serial_string("AIUEOS_VIRTIO_BLK_OK capacity-bounded sector=0 bytes=512 readonly\r\n");
    if (aiueos_virtio_blk_irq_count < 5) {
      serial_string("AIUEOS_VIRTIO_BLK_MSIX_FAIL irq-count\r\n"); qemu_exit(0x6f);
    }
    debug_string("AIUEOS_VIRTIO_BLK_MSIX_OK vector=35 irq-completions-bounded table-pba-bounded\n");
    serial_string("AIUEOS_VIRTIO_BLK_MSIX_OK vector=35 irq-completions-bounded table-pba-bounded\r\n");
    if (!aiueos_object_store_ready()) {
      debug_string("AIUEOS_OBJECT_STORE_FAIL superblock-or-object\n");
      serial_string("AIUEOS_OBJECT_STORE_FAIL superblock-or-object\r\n");
      qemu_exit(0x70);
    }
    debug_string("AIUEOS_OBJECT_STORE_OK aiuefs-v1 objects=1 checksum=fnv1a\n");
    serial_string("AIUEOS_OBJECT_STORE_OK aiuefs-v1 objects=1 checksum=fnv1a\r\n");
    if (!aiueos_journal_ready()) {
      debug_string("AIUEOS_JOURNAL_FAIL write-readback\n");
      serial_string("AIUEOS_JOURNAL_FAIL write-readback\r\n");
      qemu_exit(0x6f);
    }
    if (!aiueos_journal_sequence() || aiueos_journal_slot() < 1 || aiueos_journal_slot() > 2)
      qemu_exit(0x6f);
    debug_string("AIUEOS_JOURNAL_OK dual-slot committed append-readback\n");
    serial_string("AIUEOS_JOURNAL_OK dual-slot committed append-readback\r\n");
    if (aiueos_journal_recovered()) {
      if (!aiueos_journal_recovered_sequence() ||
          aiueos_journal_sequence() != aiueos_journal_recovered_sequence() + 1) qemu_exit(0x6f);
      debug_string("AIUEOS_JOURNAL_RECOVERY_OK highest-valid selected alternate-slot-append\n");
      serial_string("AIUEOS_JOURNAL_RECOVERY_OK highest-valid selected alternate-slot-append\r\n");
    }
    /* The input result bit is set only after a validated event has been copied
       into the browser envelope; no second mutable readiness check is needed. */
    if (!(pci_result & 4)) {
      serial_string("AIUEOS_VIRTIO_INPUT_FAIL queue-or-envelope\r\n"); qemu_exit(0x6f);
    }
    debug_string("AIUEOS_VIRTIO_INPUT_OK modern-pci eventq configured synthetic-smoke\n");
    serial_string("AIUEOS_VIRTIO_INPUT_OK modern-pci eventq configured synthetic-smoke\r\n");
    debug_string("AIUEOS_DESKTOP_INPUT_OK envelope-v1 sequence=1 kind=key ime-neutral\n");
    serial_string("AIUEOS_DESKTOP_INPUT_OK envelope-v1 sequence=1 kind=key ime-neutral\r\n");
    debug_string("AIUEOS_SCHEDULER_OK tasks=2 policy=round-robin preemption=apic-timer\n");
    serial_string("AIUEOS_SCHEDULER_OK tasks=2 policy=round-robin preemption=apic-timer\r\n");
    debug_string("AIUEOS_SCHEDULER_CR3_OK roots=3 private-pages=2 kernel-return\n");
    serial_string("AIUEOS_SCHEDULER_CR3_OK roots=3 private-pages=2 kernel-return\r\n");
    if (!aiueos_ioapic_route_legacy_timer()) {
      debug_string("AIUEOS_IOAPIC_FAIL route-legacy-timer\n");
      serial_string("AIUEOS_IOAPIC_FAIL route-legacy-timer\r\n");
      qemu_exit(0x72);
    }
    __asm__ volatile("sti");
    while (aiueos_external_timer_ticks == 0) __asm__ volatile("hlt");
    __asm__ volatile("cli");
    debug_string("AIUEOS_IOAPIC_OK pit-gsi vector=33 eoi-v1\n");
    serial_string("AIUEOS_IOAPIC_OK pit-gsi vector=33 eoi-v1\r\n");
    if (!aiueos_syscall_self_test()) {
      debug_string("AIUEOS_SYSCALL_FAIL abi-capability-pointer\n");
      serial_string("AIUEOS_SYSCALL_FAIL abi-capability-pointer\r\n");
      qemu_exit(0x73);
    }
    debug_string("AIUEOS_SYSCALL_OK int80-cpl0 abi-v1\n");
    serial_string("AIUEOS_SYSCALL_OK int80-cpl0 abi-v1\r\n");
    debug_string("AIUEOS_CAPABILITY_OK handle-v1 invalid-handle-denied\n");
    serial_string("AIUEOS_CAPABILITY_OK handle-v1 invalid-handle-denied\r\n");
    debug_string("AIUEOS_COPYIN_OK noncanonical-and-unmapped-denied\n");
    serial_string("AIUEOS_COPYIN_OK noncanonical-and-unmapped-denied\r\n");
    int process_init = aiueos_process_initialize();
    if (process_init != 1) {
      serial_string("AIUEOS_RING3_FAIL tss-or-mapping\r\n"); qemu_exit(0x72);
    }
    debug_string("AIUEOS_PROCESS_FOUNDATION_OK tss-descriptor user-wx guard-page\n");
    serial_string("AIUEOS_PROCESS_FOUNDATION_OK tss-descriptor user-wx guard-page\r\n");
    aiueos_load_task_register();
    aiueos_process_enter();
    if (!aiueos_process_result()) {
      debug_string("AIUEOS_RING3_FAIL syscall-results\n");
      serial_string("AIUEOS_RING3_FAIL syscall-results\r\n");
      qemu_exit(0x71);
    }
    debug_string("AIUEOS_RING3_OK cpl3-int80 tss-rsp0 return-kernel\n");
    serial_string("AIUEOS_RING3_OK cpl3-int80 tss-rsp0 return-kernel\r\n");
    debug_string("AIUEOS_USER_SYSCALL_OK valid-log invalid-handle invalid-pointer\n");
    serial_string("AIUEOS_USER_SYSCALL_OK valid-log invalid-handle invalid-pointer\r\n");
    if (!aiueos_address_space_self_test()) {
      debug_string("AIUEOS_ADDRESS_SPACE_FAIL cr3-or-isolation\n");
      serial_string("AIUEOS_ADDRESS_SPACE_FAIL cr3-or-isolation\r\n");
      qemu_exit(0x76);
    }
    debug_string("AIUEOS_ADDRESS_SPACE_OK processes=2 distinct-cr3 private-pages cross-access-fault\n");
    serial_string("AIUEOS_ADDRESS_SPACE_OK processes=2 distinct-cr3 private-pages cross-access-fault\r\n");
    aiueos_page_fault_stage = 1;
    aiueos_probe_write_protect();
    if (aiueos_page_fault_stage != 0x101 ||
        (aiueos_page_fault_error & 0x3) != 0x3) {
      debug_string("AIUEOS_PAGE_FAULT_FAIL write-protect\n");
      serial_string("AIUEOS_PAGE_FAULT_FAIL write-protect\r\n");
      qemu_exit(0x7a);
    }
    debug_string("AIUEOS_PAGE_FAULT_OK write-protect vector=14\n");
    serial_string("AIUEOS_PAGE_FAULT_OK write-protect vector=14\r\n");
    aiueos_page_fault_stage = 2;
    aiueos_probe_no_execute();
    if (aiueos_page_fault_stage != 0x102 ||
        (aiueos_page_fault_error & 0x11) != 0x11 ||
        (aiueos_page_fault_error & 0x2) != 0) {
      debug_string("AIUEOS_PAGE_FAULT_FAIL no-execute\n");
      serial_string("AIUEOS_PAGE_FAULT_FAIL no-execute\r\n");
      qemu_exit(0x79);
    }
    debug_string("AIUEOS_PAGE_FAULT_OK no-execute vector=14\n");
    serial_string("AIUEOS_PAGE_FAULT_OK no-execute vector=14\r\n");
    __asm__ volatile("ud2");
  }
  for (;;) __asm__ volatile("cli; hlt");
}
