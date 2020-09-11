# Quickstart

**`clicky` is not ready for general use yet!** This quickstart guide is aimed at _developers_.

`clicky` is split up into multiple crates:

| crate            | type |                                                                                  |
| ---------------- | ---- | -------------------------------------------------------------------------------- |
| `clicky-core`    | lib  | Platform agnostic emulator code.                                                 |
| `relativity`     | lib  | Cross-platform timers and `Instant` which can be paused/resumed/shifted in time. |
| `clicky-desktop` | bin  | A native CLI + GUI to interact with `clicky-core`.                               |
| `clicky-web`     | bin  | Run `clicky` on the web using the power of `wasm`! (_very_ WIP)                  |

At the moment, the recommended frontend to use is **`clicky-desktop`**.

See the `README.md` files under the `clicky-desktop` and `clicky-web` directories for build instructions.

See `ARCHITECURE.md` for a more detailed overview of how `clicky`'s source code is structured.

_Note:_ `clicky` is primarily developed and tested on Linux, though it is being written with cross-platform support in mind. At some point, I do intend to set up a CI to ensure `clicky` compiles on Windows/macOS, but until that point, please file an issue if `clicky` doesn't compile on your system.

_Note:_ All scripts and snippets below assume you're running a Unix-like environment. If you're on Windows, I recommend using WSL to run the various scripts mentioned below.

## Obtaining iPod software

What good is an emulator without any software?

Unfortunately, getting iPod software isn't super simple, and wrangling it into the right format to work with `clicky` can be kinda tricky.

### Creating a blank HDD image

`scripts/rawhd/make_rawhd.sh` is used to create a bare-bones iPod disk image for testing and development. The resulting disk image is only 64MiB in size, and uses WinPod formatting (MBR). It contains two partitions: an iPod firmware partition, and a FAT32 partition.

Getting data onto the disk image is a bit finicky. On Linux, you can run `sudo mount -o loop,offset=$((12288 * 512)) ipodhd.img tmp/` to mount the FAT32 partition. The specific offset number corresponds to the location of the FAT32 partition in the disk image, which can be determined by running `fdisk -lu ipodhd.img`. Alternatively, you can use `mtools` to copy files/folders over without having to mount the image file. Check out the various scripts under `scripts/rawhd` for examples of how to manipulate data on the disk image.

`scripts/rawhd/make_rawhd.sh` accepts a single argument: a path to a iPod firmware file. If no firmware file is provided, the firmware partition will be left empty.

### Building + Running some test firmwares

I've included the source of `ipodloader` and `ipodloader2` in-tree under `./resources/`, and fixed-up their makefiles / sources to compile under more recent gcc toolchains (namely: `gcc-arm-none-eabi`). Additionally, I've tweaked some compiler flags to disable optimizations + enable debug symbols, which should make debugging a lot easier.

These test images doesn't really do much, as `loop.bin` is simply a placeholder which loops forever once it's loaded. That said, these images can serve as a good smoke tests to check if various bits of hardware are working as intended.

Once the correct toolchain is installed, you can build some iPod firmware images based on `ipodloader` and `ipodloader2` by running:

```bash
# ipodloader test firmware
cd ./resources/ipodloader
make
./make_fw -v -g 4g -o ipodloader_loops_unopt.bin -l ../loop.bin -l ../loop.bin loader.bin
cd ../../
# ipodloader2 test firmware. reuses the `make_fw` utility from ipodloader
cd ./resources/ipodloader2
make
../ipodloader/make_fw -v -g 4g -o ipodloader2_loop.bin -l ../loop.bin loader.bin
```

Additionally, `ipodloader2` requires a valid HDD to be present:

```bash
# creates an `ipodhd.img` raw disk image with `ipodloader2_loop.bin`
./scripts/rawhd/make_rawhd.sh ./resources/ipodloader2/ipodloader2_loop.bin
./scripts/rawhd/add_ipodloader_cfg.sh # enables debug output, so it's not just a white screen
```

With the images built, it should be possible to run them in `clicky`!

e.g: using `clicky-desktop`:

```bash
cargo run -p clicky-desktop --release -- --hle=./resources/ipodloader/ipodloader_loops_unopt.bin --hdd=null:len=1GiB
cargo run -p clicky-desktop --release -- --hle=./resources/ipodloader2/ipodloader2_loop.bin --hdd=raw:file=ipodhd.img
```

`ipodloader_loops_unopt.bin` should display an image of the iPodLinux Tux and then loop forever. It's not really useful other than as a smoke-test to make sure `clicky` is somewhat working as intended.

`ipodloader2_loop.bin` should display a menu of various boot options. It's more complex than `ipodloader` v1, and serves as a great testbed for implementing / testing all sorts of misc ipod hardware.

### Building + Running Rockbox

[Rockbox](https://www.rockbox.org/) is an open source firmware replacement for digital music players, including the iPod.

The Rockbox documentation recommends using the `ipodpatcher` utility to install Rockbox. Unfortunately, `ipodpatcher` doesn't support writing directly to a disk image, so instead, I recommend building Rockbox + the Rockbox bootloader manually, and using the `make_fw` utility (included with the `ipoadloader` source code in-repo) to create a firmware image. The added benefit of this approach is that it's possible to compile Rockbox with debug symbols, which is incredibly helpful for debugging!

Building Rockbox from source is relatively straightforward. Just clone the repo, and follow the steps in the README. A couple of things to look out for:

-   Use `../tools/configure --compiler-prefix=arm-none-eabi-` to compile Rockbox using the modern `arm-none-eabi-` toolchain.
-   When compiling Rockbox, select the `(A)dvanced` option, and enable `(D)EBUG` and `(L)ogf`.
    -   Don't forget to run `make zip` after compiling!
-   When compiling the Rockbox bootloader, you'll have to manually edit the resulting `Makefile` to pass `-g` to the compiler to enable debug symbols.

Once the bootloader (`bootloader.bin`) and the main firmware image (`rockbox.ipod`) have been compiled successfully, you can use the `make_fw` utility to create a firmware image binary.

```bash
make_fw -v -g 4g -o rockbox_fw.bin -i rockbox.ipod bootloader.bin
```

The firmware image + rockbox.zip can then be loaded onto a HDD image:

```bash
# creates an `ipodhd.img` raw disk image with `ipodloader2_loop.bin`
./scripts/rawhd/make_rawhd.sh /path/to/rockbox_fw.bin
./scripts/rawhd/copy_rockbox.sh /path/to/rockbox.zip
```

Finally, the firmware image + disk image can be loaded into clicky:

```bash
cargo run -p clicky-desktop --release -- --hle=/path/to/rockbox_fw.bin --hdd=mem:file=ipodhd.img
```

When debugging, load debugging symbols from `bootloader.elf` and `rockbox.elf`.

## Diving in to `clicky`'s source code

Now that you've got `clicky` built and running some software, you might be interested in working on `clicky` itself. If so, check out `DEVGUIDE.md` for more info on how the project is structured, and tips on how to contribute to the project!
