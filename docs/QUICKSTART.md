# Quickstart

**`clicky` is a work-in-progress iPod emulator.** This guide is for developers who want to build `clicky`, prepare test firmware/disk images, and get an iPod workload running.

> **Platform note:** `clicky` is primarily developed and tested on Linux. The project is written with cross-platform support in mind, but Windows/macOS CI is not currently guaranteed. The commands below assume a Unix-like shell; Windows users can use WSL.

## 1. Install the prerequisites

You need:

- Rust and Cargo (install with [rustup](https://rustup.rs/))
- A working C compiler/toolchain
- Git
- `gcc-arm-none-eabi` and its binutils for building the bundled ARM test firmware
- `make`, `dd`, `sfdisk`, and `mkdosfs` for the disk-image helpers

On Debian/Ubuntu, the native libraries used by `clicky-desktop` may also require:

```bash
sudo apt install build-essential gcc-arm-none-eabi binutils-arm-none-eabi     make dosfstools util-linux libxkbcommon-dev libwayland-dev
```

Clone the repository and verify that the workspace builds:

```bash
git clone https://github.com/daniel5151/clicky.git
cd clicky
cargo build --release -p clicky-desktop
```

The release build is recommended. Debug builds are substantially slower.

## 2. Know the project layout

`clicky` is split into several crates:

| Crate | Type | Purpose |
| --- | --- | --- |
| `clicky-core` | library | Platform-agnostic emulator implementation |
| `relativity` | library | Cross-platform timing utilities |
| `clicky-desktop` | binary | Native CLI + GUI frontend |
| `clicky-web` | binary | Experimental WebAssembly frontend |

For most development work, start with **`clicky-desktop`**.

For a deeper architecture overview, see [ARCHITECURE.md](ARCHITECURE.md), and for contributor-oriented guidance see [DEVGUIDE.md](DEVGUIDE.md).

## 3. The files `clicky` needs

There are two main ways to boot software:

### HLE boot

HLE (High Level Emulation) uses a firmware binary and does **not** require an Apple Flash ROM dump.

You need:

- an iPod HDD image
- a firmware binary

This is the easiest path for development and is enough for most software.

### LLE boot

LLE (Low Level Emulation) starts from a real Flash ROM dump.

You need:

- an iPod HDD image
- a compatible Flash ROM dump from your own hardware

The Flash ROM contains copyrighted Apple code and is not distributed with `clicky`. **You do not need a Flash ROM dump to get started.**

## 4. Create a test HDD image

The repository includes helpers for creating a minimal iPod disk image.

First build one of the bundled test firmware images as described in the next section, then:

```bash
./scripts/rawhd/make_rawhd.sh ./resources/ipodloader2/ipodloader2_loop.bin
./scripts/rawhd/add_ipodloader_cfg.sh
```

This creates `ipodhd.img`.

The generated image is a 64 MiB WinPod-style MBR image containing:

- an iPod firmware partition
- a FAT32 partition

To inspect the partition layout:

```bash
fdisk -lu ipodhd.img
```

On Linux, the FAT32 partition can be mounted with:

```bash
mkdir -p tmp
sudo mount -o loop,offset=$((12288 * 512)) ipodhd.img tmp/
```

Unmount it when finished:

```bash
sudo umount tmp
```

The `scripts/rawhd/` directory also contains helpers for manipulating these images without mounting them directly.

> **Safety:** Always use an image file when experimenting. Do not substitute a real block device such as `/dev/sda` for `ipodhd.img`.

## 5. Build the bundled ARM test firmware

The repository includes source for `ipodloader` and `ipodloader2`. These are useful smoke tests because they exercise iPod-specific hardware without requiring a full operating system.

### ipodloader

```bash
cd resources/ipodloader
make
./make_fw -v -g 4g -o ipodloader_loops_unopt.bin     -l ../loop.bin -l ../loop.bin loader.bin
cd ../..
```

### ipodloader2

```bash
cd resources/ipodloader2
make
../ipodloader/make_fw -v -g 4g -o ipodloader2_loop.bin     -l ../loop.bin loader.bin
cd ../..
```

Then create the corresponding HDD image:

```bash
./scripts/rawhd/make_rawhd.sh ./resources/ipodloader2/ipodloader2_loop.bin
./scripts/rawhd/add_ipodloader_cfg.sh
```

## 6. Run `clicky`

### ipodloader smoke test

```bash
cargo run -p clicky-desktop --release -- \
    --hle=./resources/ipodloader/ipodloader_loops_unopt.bin \
    --hdd=null:len=1GiB
```

### ipodloader2 smoke test

```bash
cargo run -p clicky-desktop --release -- \
    --hle=./resources/ipodloader2/ipodloader2_loop.bin \
    --hdd=raw:file=ipodhd.img
```

The first test should display the iPodLinux Tux and then loop. The second should display the `ipodloader2` boot menu.

These are intentionally simple: their value is as **hardware smoke tests** for the emulator.

## 7. Run Rockbox

Rockbox is an open-source firmware replacement for digital music players, including the iPod.

Build Rockbox and its iPod bootloader following the Rockbox documentation. For this project, the important details are:

- use `../tools/configure --compiler-prefix=arm-none-eabi-`
- enable **DEBUG** and **LOGF** in the advanced configuration
- run `make zip` after compiling
- add `-g` to the bootloader compiler flags when debugging

Once you have `bootloader.bin` and `rockbox.ipod`:

```bash
make_fw -v -g 4g -o rockbox_fw.bin -i rockbox.ipod bootloader.bin
```

Create an HDD image and copy Rockbox onto it:

```bash
./scripts/rawhd/make_rawhd.sh /path/to/rockbox_fw.bin
./scripts/rawhd/copy_rockbox.sh /path/to/rockbox.zip
```

Run it:

```bash
cargo run -p clicky-desktop --release -- \
    --hle=/path/to/rockbox_fw.bin \
    --hdd=mem:file=ipodhd.img
```

For debugging, keep the generated `bootloader.elf` and `rockbox.elf` files so a debugger can load their symbols.

## 8. Dumping an Internal Flash ROM

A Flash ROM dump is **optional**. HLE is the recommended way to get started.

If you have a compatible iPod 4G and want to experiment with LLE, Rockbox provides a Flash ROM dump facility:

1. Boot Rockbox on the iPod.
2. Open **System → Debug (Keep Out!) → Dump ROM contents**.
3. Copy `internal_rom_000000-0FFFFF.bin` to your development machine.

Then run `clicky` with the dump:

```bash
cargo run -p clicky-desktop --release -- \
    --flash-rom=/path/to/internal_rom_000000-0FFFFF.bin \
    --hdd=mem:file=/path/to/ipodhd.img
```

With a valid Flash ROM image, the HLE bootloader can be omitted and `clicky` can perform a cold boot from the emulated Flash ROM.

Only use ROM dumps obtained from hardware you own or are otherwise authorized to access. `clicky` does not redistribute Apple's Flash ROM contents.

## 9. Common problems

### Build fails around Wayland/XKB

On Debian/Ubuntu, install:

```bash
sudo apt install libxkbcommon-dev libwayland-dev
```

Then retry:

```bash
cargo build --release -p clicky-desktop
```

### The emulator starts but the screen is blank

Start with the bundled `ipodloader2` smoke test. Also run:

```bash
./scripts/rawhd/add_ipodloader_cfg.sh
```

That enables additional debug output for the test image.

If the emulator reports accesses to an unimplemented device, that may be useful: `clicky` is an emulator under active development, and those logs often identify hardware that still needs implementation.

### I changed the HDD image and want a reproducible run

Use:

```text
--hdd=mem:file=ipodhd.img
```

The image is loaded into memory instead of being written back to disk. This is useful when repeatedly testing the same image.

### I am on Windows

The documented scripts assume a Unix-like shell. WSL is the recommended workaround until broader native platform CI is available.

### I am on macOS

The project is intended to be portable, but development and testing are primarily Linux-based. If a build or runtime dependency is platform-specific, check the relevant crate README and report a reproducible issue if the project does not build or run correctly.

## 10. Start contributing

Once you can boot a workload, read [DEVGUIDE.md](DEVGUIDE.md).

A typical development run looks like:

```bash
RUST_LOG=MMIO=info,GPIO=trace,gdbstub=error \
cargo run -p clicky-desktop --release -- \
    --hdd=mem:file=/path/to/ipodhd.img \
    --hle=/path/to/rockbox_bootloader_fw.bin \
    --flash-rom=/path/to/internal_rom_000000-0FFFFF.bin \
    -g /tmp/clicky,on-fatal-err
```

The logs are often the starting point for implementing missing or inaccurate hardware devices.

Useful areas to explore include:

- improving stubbed or partially implemented peripherals
- addressing `TODO`, `FIXME`, `HACK`, and `XXX` comments
- validating hardware behavior against a physical iPod
- improving emulator architecture
- bringing up additional iPod models

The goal of this guide is to get a new contributor from **zero → built → booting → debugging** without having to reverse-engineer the repository's setup process first.
