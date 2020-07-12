# clicky

A WIP clickwheel iPod emulator.

Here's a clip of `clicky` emulating an iPod 4G running `ipodloader2` (a third-party bootloader for the iPod) and loading [Rockbox](https://www.rockbox.org/) into system memory.

<img height="256px" src="screenshots/clicky-ipodloader2-lle.gif" alt="clicky booting ipodloader2 + rockbox">

**This project is not ready for general use yet!**

`clicky` is still in it's early stages, and there hasn't been much effort put into making it easy to use.

That said, if you're a cool [hackerman](https://www.youtube.com/watch?v=V4MF2s6MLxY) who can [jam with the console cowboys in cyberspace](https://www.youtube.com/watch?v=BNtcWpY4YLY), check out the [Quickstart](#quickstart) guide on how to build `clicky` from source.

---

Are you someone with strong **reverse engineering** experience and wants to help preserve an iconic piece of early 2000s pop-culture? If so, read on!

While I expect that I'll be able to get Rockbox and iPodLinux up and running, I worry that getting Apple's RetailOS working may prove difficult. While lots of reverse-engineering work has already been done by the iPodLinux and Rockbox projects back around 2007, it seems that there are still plenty of registers / memory blocks whose purpose is unknown. `clicky` can already boot into RetailOS, and I'm noticing lots of accesses to undocumented parts of the PP5020 memory space.

Fortunately, now that we're living in 2020 (i.e: the future), we have access to newer, better tools that can aid in reverse-engineering the iPod. Free and powerful reverse engineering tools (like [Gridra](https://ghidra-sre.org/)), and emulation software (`clicky` itself) aught to make it easier to inspect and observe the state of the RetailOS binaries while they're being run, and gain insight into what the hardware is supposed to do.

I've got some reverse engineering experience, but truth be told, it's not really my forte, so if you're interested in helping out, please get in touch!

---

## Emulated Hardware

- MVP: [iPod 4g (Grayscale)](https://everymac.com/systems/apple/ipod/specs/ipod_4thgen.html)
- End goal: [iPod 5g](https://everymac.com/systems/apple/ipod/specs/ipod_5thgen.html)

Why these models?

The 4g uses the same/similar SOC as some of the later generation models (PP5020), while using a simpler (grayscale) display. This should make it easier to get display emulation up and running, leaving more time to implement other devices.

The 5g is the first iPod model to support [iPod Games](https://en.wikipedia.org/wiki/IPod_game), which are an interesting part of gaming history which have never been preserved!

## Quickstart

`clicky` is primarily developed and tested on Linux, though it is being written with cross-platform support in mind. At some point, I do intend to set up a CI to ensure `clicky` compiles on Windows/macOS, but until that point, please file an issue if `clicky` doesn't compile on your system.

_Note:_ All scripts and snippets below assume you're running a Unix-like environment. If you're on Windows, I recommend using WSL to create/manipulate iPod disk images.

### Building `clicky`

`clicky` uses the standard `cargo` build flow, so running `cargo build --release` should do the trick.

```bash
git clone https://github.com/daniel5151/clicky.git
cd clicky
cargo build --release
```

#### Common Build Errors

- (Linux) You may encounter some build-script / linker errors related to missing `xkbcommon` and `wayland` libraries. On Debian/Ubuntu, you can install them via `apt install libxkbcommon-dev libwayland-dev`.

### Building a test firmware

While the end-goal is to boot into Rockbox, iPodLinux, and Apple's RetailOS, `clicky` isn't quite there yet.

I've included the source of `ipodloader` and `ipodloader2` in-tree under `./resources/`, and fixed-up their makefiles / sources to compile under more recent gcc toolchains (namely: `gcc-arm-none-eabi`). Additionally, I've tweaked some compiler flags to disable optimizations + enable debug symbols, which should make debugging a lot easier.

On Debian/Ubuntu based distros, you can install the correct toolchain via `apt install gcc-arm-none-eabi`.

Once the correct toolchain is installed, you can build some iPod firmware binaries based on `ipodloader` and  `ipodloader2` by running:

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

### Creating a HDD image

To get up-and-running with a basic test image, run the following commands:

```bash
./scripts/mkipodhd_raw.sh ./resources/ipodloader2 # creates an `ipodhd.img` raw disk image
./scripts/add_ipodloader_cfg_to_rawhd.sh
```

The resulting disk image is only 64MiB in size. It uses WinPod formatting (MBR) with two partitions: an iPod firmware partition, and a FAT32 partition.

Getting data onto the disk image is a bit finicky. On Linux, you can run `sudo mount -o loop,offset=$((12288 * 512)) ipodhd.img tmp/` to mount the FAT32 partition. The specific offset number corresponds to the location of the FAT32 partition in the disk image. The offset value is determined by running `fdisk -lu ipodhd.img`.

Alternatively, you can use `mtools` to copy files/folders over without having to mount the image file. See the `add_ipodloader_cfg_to_rawhd.sh` script for an example of this.

### Running `clicky`

Now that you have some iPod firmware images, you can finally run clicky:

```bash
cargo run -- ./resources/ipodloader/ipodloader_loops_unopt.bin --hdd=null:len=1GiB # doesn't require a hdd image
cargo run -- ./resources/ipodloader2/ipodloader2_loops.bin --hdd=raw:file=ipodhd.img
```

`ipodloader_loops_unopt.bin` should display an image of the iPodLinux Tux and then loop forever. It's not really useful other than as a smoke-test to make sure clicky built correctly.

`ipodloader2_loops.bin` is what's currently being worked on, and output fluctuates every other commit :)

## Dev Guide

**Disclaimer:** `clicky` is still in the "one man project" phase of development, where I'm pushing to master, and every third commit re-architects some fundamental aspects of the emulator's framework. If you interested, I'd recommend holding off on opening a PR until things stabilize a bit more (and I don't need to have this disclaimer anymore).

### Diving in

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

Poking around the PP2050 documentation indicates that address `0x70003000` has something to do with the LCD controller.

### Debugging using GDB

If you happen to be running code that's been compiled with debug symbols, you can use `gdb-multiarch` (or `gdb-arm-none-eabi`) to poke around the offending code. Run `clicky` with the `-g` flag to spawn a local gdb server, and connect to it from `gdb` by running `target remote :9001`. See the `.gdbinit` files under the `ipodloader` directory for more details.

`clicky` exposes additional custom debugging features using GDB's `monitor` command. Invoking `monitor help` will list available monitor commands.

### Resources

Any useful resources I stumble across during development are stashed away under the `resources` folder. You'll find various technical reference manuals, spec sheets, and iPod-related utilities. `resources/documentation/LINKS.md` links to additional online resources.

### Using `clicky` with an Apple flash ROM dump

A proper Low Level Emulation (LLE) boot process would involve booting the CPU from address 0 and having it execute whatever bootloader code is present in Flash ROM. This code includes some basic device setup, toggling certain interrupts, and of course, loading the actual firmware image from the HDD into executable memory.

The code contained in Flash ROM is copyrighted by Apple, and as such, `clicky` cannot legally redistribute any copies of it. To work around this, `clicky` includes a High Level Emulation (HLE) bootloader. `clicky` will manually set the state of certain devices, toggle certain interrupts, load the firmware image into memory, and set the Program Counter to the load address of the firmware image. If any code attempts to access the memory locations mapped to flash ROM (e.g: to determine which iPod version it's running on), the `Flash` device implements a limited set of memory locations which can be accessed to complete initialization.

As the project matures, it should be possible to perform a "cold boot" using a dumped Flash ROM image. This setting is supported via the `--flash-rom` flag.

**NOTE:** At this stage in development, having a Flash ROM image is **not** required to run `clicky`. I plan to maintain the HLE bootloader so that `clicky` is as easy to use as possible.

That said, if you're interested in helping out with `clicky`'s development, you might want to try to get your hands on a flash ROM image. If you happen to still have an old iPod lying around, you can dump the contents of it's flash ROM using Rockbox, as described [here](https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code).

## Roadmap

The plan is to implement devices and hardware "just in time" throughout development, instead of attempting to one-shot the entire SOC right off the bat. As such, the idea is to gradually test more and more complex software in the emulator, implementing more and more hardware as required.

_Note:_ This roadmap was written fairly early in the project's development, and hasn't been updated in a while. It's still mostly accurate, though in hindsight, it seems to under/overestimate how complicated certain features are to implement.

- [x] Execute something _really_ basic, such as https://github.com/iPodLinux/ipodloader/
    - This rough-little bit of software is simple enough to step through and understand fully, making it a great launching off point for the project.
    - It touches quite a bit of iPod-specific hardware (e.g: Timers, Buttons, LCD)
    - **Goals:**
        - Find my footing with the ARM7TDMI CPU, and the iPod's funky dual-processor architecture
        - Get more familiar with the ARM7TDMI assembler and compiler toolchain
        - Set up project boilerplate
            - Memory interconnect framework
            - LCD output, button input
            - basic CLI
        - Scaffold basic system architecture (step through CPU, system memory map, interact with devices)
- [x] Get through the more complex https://github.com/iPodLinux/ipodloader2/
    - Touches even _more_ iPod-specific hardware (ATA-2)
    - Seems to do more in-depth system init (i.e: interrupt handling, memory mapping)
    - **Goals:**
        - Expand on the system architecture + implemented devices
- [ ] Boot / pass the Apple Diagnostics program
    - If you press and hold the Select+Prev while an iPod is booting up, a diagnostics program built directly into the Flash ROM is executed!
    - This would likely be the first closed source software the emulator runs.
    - Makes for a great playground to poke at the various hardware features that exist on the iPod, without worrying too much about an OS scheduler getting in the way.
    - **Goals:**
        - Implement even more devices
- [ ] Boot into [Rockbox](https://www.rockbox.org/)
    - A gargantuan task, one which will involve implementing a _lot_ of misc. hardware
    - Since the OS is open source, is should be possible to trace through the code, making debugging a lot easier.
    - **Goals:**
        - Boot an actual OS on the iPod
- [ ] Boot into [iPod Linux](http://www.ipodlinux.org/)
    - A bigger beast than Rockbox, and likely much more difficult to step through and debug
    - **Goals:**
        - Booth _another_ actual OS on the iPod
        - Fill in the gaps between the hardware Rockbox uses, and the hardware iPod Linux uses
- [ ] Boot into RetailOS
    - i.e: _the big money goal_
    - Hopefully, by getting two other OSs up and running, RetailOS will "just work"
    - Realistically, those Apple engineers probably did some fancy/janky stuff, and things will be very broken
    - **Goals:**
        - Get an actual working emulated iPod up and running!
        - Play some authentic Brick Breaker!

Once things seem stable, it shouldn't be _too_ difficult to get the iPod 5g up and running, since it's mostly the same hardware, mod the color screen.

### Unknowns that might make things tricky

- Funky cache effects
    - I _really_ don't want to deal with implementing proper caching if I don't have to. I'm gonna cross my fingers, and hope that having both CPUs see memory writes at the same time will be _fiiiiine_
- Funky iPod hardware that _hasn't_ been reverse engineered
    - ...this will suck, and unfortunately, It's probably something I'll encounter once I start messing around with RetailOS.

### Things probably best left for later

- USB
    - This seems like a huge rabbit hole of complexity, and is something that probably isn't critical to the iPod's core functions. Stubbing things out will probably be fine...
- Audio
    - inb4 "but it's an iPod, it's literally an _audio player_"
    - yeah, I know, but Audio is hard and finicky to get right, so I'll be leaving it for _waaaaaay_ later

---

## Fluff: Why emulate the iPod?

'cause it's a neat technical challenge lol.

Compared to my last big emulation project ([ANESE](https://prilik.com/ANESE), a NES emulator that [automatically maps out NES games](https://prilik.com/blog/wideNES)), the iPod presents a totally different set of technical challenges to overcome.

First of all, the iPod is a fairly modern system. Unlike the esoteric and custom-made chips used in many game consoles, the iPod uses many off-the-shelf commodity hardware and technologies. As such, this project should be a good way to explore and learn more about the low level details of the ARM architecture, I2S, I2C, USB, IDE HDDs, etc...

Second of all, the iPod isn't very well documented! While this'll probably end up being more annoying than exciting in the long run, I'm excited to do my own research, discover new information, and consolidate information on the iPod myself (as opposed to already having a well organized and complete reference at my disposal \*cough\* the nesdev wiki \*cough\*). As it turns out, there's already quite a amount of documentation about the iPod that's floating around (thanks to the iPodLinux and Rockbox projects), but I'm sure there will still be plenty of stuff left for me to discover. Time to finally learn how to use [Ghidra](https://ghidra-sre.org/) I guess!

Lastly, the iPod is a system that's never been emulated before! That means there usually won't be any sort of "escape hatch" when I get stuck, since there's no one else's code I can peek at. Whatever challenges I run in to will be challenges I'm going to have to solve myself! How exciting!

...there is one last reason I want to emulate the iPod though:

**It's got _Brick Breaker!_**

> _ooooooh Brick Breaker baybeeeeee! This game has won game of the year, I don't know how many times!_

But seriously, aside from brick breaker, there were actually a whole bunch of [iPod Games](https://en.wikipedia.org/wiki/IPod_game) released for late-gen iPod models \~2006. While these games aren't necessarily _masterpieces_, they're still pretty neat, and aught to be preserved.

In fact, my initial inspiration for starting this project was actually hearing about these old games, and how no one has ever looked into preserving them. While getting these games working will probably take quite a while, it's a neat long-term goal to aim for.

## Thanks and Acknowledgments

This project would be dead in the waters without these folks and projects:

- [The iPod Linux Project](http://www.ipodlinux.org/) - for invaluable iPod reverse engineering work
- [Rockbox](https://www.rockbox.org/) - for additional iPod reverse engineering work (and preserving _years_ of IRC logs to search through)
- [QEMU](https://www.qemu.org/) - for insights on how to structure the codebase, and how certain devices aught to work
- [Sean Purcell](https://github.com/iburinoc/) - for writing the bulk of [armv4t_emu](https://github.com/daniel5151/armv4t_emu)
