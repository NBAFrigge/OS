# Phase 1: Interrupts and CPU Exceptions

- **IDT Implementation:** Define the Interrupt Descriptor Table to catch CPU exceptions (Divide-by-zero, Page Fault, Double Fault).
- **External Interrupts:** Configure the Programmable Interrupt Controller (PIC) or APIC to handle hardware signals.
- **PS/2 Keyboard Driver:** Implement an interrupt-based driver to process keyboard input and verify interrupt stability.

# Phase 2: Memory Management

- **Physical Frame Allocator:** Create a system to track used and free physical memory pages (RAM) using a bitmap or free list.
- **Paging:** Set up 4-level page tables to manage virtual memory and protect kernel space.
- **Kernel Heap:** Implement a `LockedHeap` and link the `alloc` crate to enable dynamic data structures like `Vec<u8>` and `Box`.

# Phase 3: Hardware Discovery and Bus Access

- **PCI Enumeration:** Scan the Peripheral Component Interconnect bus to locate the Network Interface Card (NIC), such as the Intel E1000.
- **Memory Mapped I/O (MMIO):** Map the NIC’s configuration registers into virtual memory.
- **System Timer:** Initialize the PIT (Programmable Interval Timer) or APIC timer for time-sensitive network operations and timeouts.

# Phase 4: Network Interface Card Driver

- **NIC Initialization:** Set up the hardware registers, transmit (TX) and receive (RX) rings.
- **DMA Buffer Management:** Allocate contiguous physical memory for Direct Memory Access (DMA) so the hardware can move packets without CPU intervention.
- **Raw Packet I/O:** Implement functions to send and receive raw byte arrays (`&[u8]`) via the hardware descriptors.

# Phase 5: Minimal Network Stack (L2 - L3)

- **Ethernet Layer:** Implement a parser and builder for Ethernet frames, including MAC address filtering.
- **ARP (Address Resolution Protocol):** Implement an ARP cache and responder so the host machine can resolve the OS's MAC address.
- **IPv4 Layer:** Handle basic IP header parsing, including version verification and checksum validation.

# Phase 6: ICMP and Ping Response

- **ICMP Parser:** Identify `Echo Request` packets (Type 8).
- **Echo Reply Logic:** Implement a responder that swaps source/destination IP and MAC addresses, changes the ICMP type to `Echo Reply` (Type 0), and recomputes the checksum.
- **Integration Testing:** Use QEMU’s `.pcap` dump feature to verify packet integrity in Wireshark.
