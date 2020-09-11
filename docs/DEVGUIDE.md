# Dev Guide

Please read `QUICKSTART.md` first. After all, it might be a bit tricky working on `clicky` if you haven't got any software to test it with!

This document provides some high-level tips for working on `clicky`. For more detailed, source-code oriented documentation, check out `ARCHITECURE.md` and `REPO_LAYOUT.md`.

## Diving in

If you're a "gung-ho" kinda person and want to get your hands dirty, your best bet would be to load up a firmware image, hit run, and implement whatever stubbed out and/or unimplemented hardware you see in the logs! If you're lucky, the emulator try to access a totally unimplemented device, which halts emulation entirely!

A typical dev-cycle run of `clicky` might look something like this:

```bash
RUST_LOG=MMIO=info,GPIO=trace,gdbstub=error             \
cargo run -p clicky-desktop --release --                \
    --hdd=mem:file=/path/to/ipodhd.img                  \
    --hle=/path/to/rockbox_bootloader_fw.bin            \
    --flash-rom=/path/to/internal_rom_000000-0FFFFF.bin \
    -g /tmp/clicky,on-fatal-err
```

-   The `RUST_LOG` environment variable is used to tweak log levels for various emulator subsystems.
    -   Depending on what you're working on, you may want to change `MMIO=trace` for even more detailed logs.
-   `--hdd=mem` indicates that the image should be loaded into memory, and _not_ written back to disk. This is useful for ensuring reproducible runs.
-   Uses the HLE bootloader alongside a real flash-rom dump (if applicable)
-   Spawn a GDB server if a fatal error occurs.
-   Run the GDB server over a Unix Domain Sockets (/tmp/clicky).
    -   Connect to the server using the GDB command `target remote /tmp/clicky`.

If all goes well, you should see a window pop up, and have your terminal be filled will a mess of logs!

At this point, it's up to you what to help with:

-   **Improving the accuracy of stubbed / partially-implemented devices**
    -   The vast majority of debug logs are related to accessing stubbed out / partially-implemented peripheral devices. It never hurts to have more accurate device implementations!
    -   Plus, often times it turns out the un-stubbing a device ends up fixing some unrelated bug, which is great!
-   **Addressing `TODO/XXX/FIXME/HACK` comments in the codebase**
    -   Running `git grep -C1 -EI "TODO\??|FIXME\??|HACK\??|XXX\??|unimplemented|todo"` should list a bunch of places where `clicky` could be improved.
    -   Many of these comments have to do with little hacks / shortcuts taken to get some code up and running. Typically, fixing these comments would improve `clicky`'s overall quality and accuracy
    -   There are also quite a few comments which document various "guesses" that have been made about the PortalPlayer SoC, which would really benefit from being hardware validated. If you have an iPod that you can run some tests on, addressing these comments would be incredibly helpful!!

For the more ambitious contributor, consider:

-   **Improving `clicky`'s high-level Architecture**
    -   As touched upon in the `ARCHITECTURE.md` document, there are a few parts of `clicky`'s high-level architecture that could use some re-engineering. For example, the DMA subsystem is a real mess (at the time of writing)
    -   While these changes don't necessarily result in an immediate accuracy improvement, they are crucial to ensuring that the `clicky` codebase stays clean, maintainable, and performant
-   **Bringing-up Additional iPod models (e.g: iPod 5g)**
    -   `clicky` is structured such that it should be fairly easy to implement different iPod models under a single codebase
    -   This would involve some plumbing work, and likely require implementing some additional devices (e.g: the iPod 5g's color display)

## Debugging using GDB

If you're running code that's been compiled with debug symbols, you can use `gdb-multiarch` (or `gdb-arm-none-eabi`) to debug the code while it's running (or in some cases, after the system has fatally crashed). Run `clicky-desktop` with the `-g` flag to spawn a local gdb server, and connect to it from `gdb` by running `target remote :9001`. See the `.gdbinit` files under the `ipodloader` directory for more details.

`clicky` exposes additional custom debugging features using GDB's `monitor` command. Running `monitor help` from the GDB prompt will list available monitor commands.

## Resources

Any useful resources I stumble across during development are stashed away under the `resources` folder. You'll find various technical reference manuals, spec sheets, and iPod-related utilities. `resources/documentation/LINKS.md` links to additional online resources.

## Using `clicky` with an Apple flash ROM dump

A proper Low Level Emulation (LLE) boot process would involve booting the CPU from address 0 and having it execute whatever bootloader code is present in Flash ROM. This code includes some basic device setup, toggling certain interrupts, and of course, loading the actual firmware image from the HDD into executable memory.

The code contained in Flash ROM is copyrighted by Apple, and as such, `clicky` cannot legally redistribute any copies of it. To work around this, `clicky` includes a High Level Emulation (HLE) bootloader. `clicky` will manually set the state of certain devices, toggle certain interrupts, load the firmware image into memory, and set the Program Counter to the load address of the firmware image. If any code attempts to access the memory locations mapped to flash ROM (e.g: to determine which iPod version it's running on), an incredibly simple HLE flash device implements a few hard-coded addresses required to continue execution.

That said, if you happen to have an old iPod 4G lying around, it's possible to dump a copy of it's Flash ROM (as described [here](https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code)), which can be passed to `clicky` via the `--flash-rom` flag. If a valid Flash ROM image is detected, the `--hle` flag can be omitted, and `clicky` will perform a proper "cold boot" using the dumped Flash ROM.

**NOTE:** At this stage in development, having a Flash ROM image is **not** required to run `clicky`! It should be possible to run most\* iPod software using the HLE bootloader.

\* While there's nothing stopping software from accessing the Flash ROM post-initialization (e.g: Rockbox includes a utility to dump the Flash ROM), there doesn't seem to be anything particularly "useful" on the Flash ROM that software would want to access.
