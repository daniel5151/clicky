# Dev Guide

**Disclaimer:** `clicky` is still in the "one man project" phase of development, where I'm pushing to master, and every third commit re-architects some fundamental aspects of the emulator's framework. If you interested in helping out, I'd recommend holding off on opening PRs until things stabilize a bit more (and I don't need to have this disclaimer anymore).

TODO: flesh this out, document internal systems + code patterns

## Diving in

If you're a "gung-ho" kinda person and want to get your hands dirty, your best bet would be to load up a firmware image, hit run, and implement whatever stubbed out and/or unimplemented hardware you see in the logs! If you're lucky, the emulator try to access a totally unimplemented device, which halts emulation entirely!

E.g: Say you run `clicky`, and you see something that looks like this (messages are very subject to change):

```
FatalAccessViolation(
    AccessViolation {
        label: "<unmapped memory>",
        addr: 0x70003000,
        kind: Unimplemented,
    },
)
```

Poking around the PP5020 documentation indicates that address `0x70003000` has something to do with the LCD controller.

## Debugging using GDB

If you happen to be running code that's been compiled with debug symbols, you can use `gdb-multiarch` (or `gdb-arm-none-eabi`) to poke around the offending code. Run `clicky-desktop` with the `-g` flag to spawn a local gdb server, and connect to it from `gdb` by running `target remote :9001`. See the `.gdbinit` files under the `ipodloader` directory for more details.

`clicky` exposes additional custom debugging features using GDB's `monitor` command. Invoking `monitor help` will list available monitor commands.

## Resources

Any useful resources I stumble across during development are stashed away under the `resources` folder. You'll find various technical reference manuals, spec sheets, and iPod-related utilities. `resources/documentation/LINKS.md` links to additional online resources.

## Using `clicky` with an Apple flash ROM dump

A proper Low Level Emulation (LLE) boot process would involve booting the CPU from address 0 and having it execute whatever bootloader code is present in Flash ROM. This code includes some basic device setup, toggling certain interrupts, and of course, loading the actual firmware image from the HDD into executable memory.

The code contained in Flash ROM is copyrighted by Apple, and as such, `clicky` cannot legally redistribute any copies of it. To work around this, `clicky` includes a High Level Emulation (HLE) bootloader. `clicky` will manually set the state of certain devices, toggle certain interrupts, load the firmware image into memory, and set the Program Counter to the load address of the firmware image. If any code attempts to access the memory locations mapped to flash ROM (e.g: to determine which iPod version it's running on), an incredibly simple HLE flash device implements a few hard-coded addresses required to continue execution.

That said, if you happen to have an old iPod 4G lying around, it's possible to dump a copy of it's Flash ROM (as described [here](https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code)), which can be passed to `clicky` via the `--flash-rom` flag. If a valid Flash ROM image is detected, the `--hle` flag can be omitted, and `clicky` will perform a proper "cold boot" using the dumped Flash ROM.

**NOTE:** At this stage in development, having a Flash ROM image is **not** required to run `clicky`! It should be possible to run most\* iPod software using the HLE bootloader.

\* While there's nothing stopping software from accessing the Flash ROM post-initialization (e.g: Rockbox includes a utility to dump the Flash ROM), there doesn't seem to be anything particularly "useful" on the Flash ROM that software would want to access.
