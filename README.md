# SwagOS

Simple OS

# Setup and run

### 1. Toolchain Setup

To compile for bare metal, you need the Rust source code and the LLVM tools for cross-compilation.

```bash
rustup component add rust-src
rustup component add llvm-tools-preview

```

### 2. Bootimage Tool

Install the `bootimage` tool to handle the creation of a bootable disk image from your compiled kernel.

```bash
cargo install bootimage

```

### 3. Build Process

Compile the kernel and link it with the bootloader to generate a bootable `.bin` image.

```bash
cargo bootimage

```

### 4. Running with QEMU

To run the OS use the following command:

```bash
qemu-system-x86_64 -drive format=raw,file=target/os/debug/bootimage-OS.bin -serial stdio

```
