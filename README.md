# clicky

A WIP clickwheel iPod emulator.

## Target Hardware

- MVP: [iPod 4g (Grayscale)](https://everymac.com/systems/apple/ipod/specs/ipod_4thgen.html)
- End goal: [iPod 5g](https://everymac.com/systems/apple/ipod/specs/ipod_5thgen.html)

Why these models?

The 4g uses the same/similar SOC as some of the later generation models (PP5020), while using a simpler (grayscale) display. This should make it easier to get display emulation up and running, leaving more time to implement other devices.

The 5g is the first iPod model to support [iPod Games](https://en.wikipedia.org/wiki/IPod_game), which are an interesting part of gaming history which have never been preserved!

## Quickstart

### Building `clicky`

`clicky` uses the standard `cargo` build flow, so running `cargo build --release` should do the trick.

```bash
git clone https://github.com/daniel5151/clicky.git
cd clicky
cargo build --release
```

When building on Linux, you may encounter some build-script / linker errors related to missing `xkbcommon` and `wayland` libraries. On Debian/Ubuntu, you can install them via `apt install libxkbcommon-dev libwayland-dev`.

### Building a test firmware

While the end-goal is to boot into Rockbox, iPodLinux, and Apple's RetailOS, `clicky` isn't quite there yet.

At the time of writing, `clicky` is (slowly but surely!) implementing the devices / features necessary to run `ipodloader2`, a third-party bootloader for the iPod. It is already capable of executing the original `ipodloader`.

I've included the source of `ipodloader` and `ipodloader2` in-tree under `./resources/`, and fixed-up their makefiles / sources to compile under more recent gcc toolchains (namely: `gcc-arm-none-eabi`). Additionally, I've tweaked some compiler flags to disable optimizations + enable debug symbols, which should makes debugging a lot easier.

On Debian/Ubuntu based distros, you can install the correct toolchain via `apt install gcc-arm-none-eabi`

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

### Running `clicky`

Now that you have some iPod firmware images, you can finally run clicky:

```bash
cargo run -- ./resources/ipodloader/ipodloader_loops_unopt.bin
cargo run -- ./resources/ipodloader2/ipodloader2_loops.bin
```

## Dev Guide

If you're interested in helping preserve a piece of iconic hardware from the early 2000s, you're more than welcome to lend a hand!

**Disclaimer:** `clicky` is still in the "one man project" phase of development, where I push to master, and every third commit is re-architecting fundamental aspects of the emulator's framework. If you interested, I'd recommend holding opening a PR for a while. I expect things to stabilize sometime around Rockbox starts working okay (namely, i've come up with a good way of handling virtual memory + interrupts).

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

If you happen to be running code that's been compiled with debug symbols, you can use `gdb-multiarch` to poke around the offending code. Run `clicky` with the `-g` flag to spawn a local gdb server, and connect to it from `gdb-multiarch` by running `target remote :9001`. See the `.gdbinit` files under the `ipodloader` directory for more details.

### Resources

Any useful resources I stumble across during development are stashed away under the `resources` folder. You'll find various technical reference manuals, spec sheets, and iPod-related utilities. `resources/documentation/LINKS.md` links to additional online resources.

### Using `clicky` with an Apple flash ROM dump

**NOTE:** This feature is currently unimplemented. I'll re-implement it once the rest of the project matures.

A proper Low Level Emulation (LLE) boot process would involve booting the CPU from address 0 and having it execute whatever bootloader code is present in Flash ROM. This code most likely performs several different functions, including setting up devices, toggling certain interrupts, and of course, loading the actual firmware image from the emulated HDD into executable memory.

The code contained in Flash ROM is copyrighted by Apple, and as such, `clicky` can't legally redistribute copies of it. Instead, `clicky` currently uses a High Level Emulation (HLE) approach to boot firmware images, where `clicky` "fakes" the Flash ROM bootloader by manually loading the firmware image into memory, and starting execution from whatever address the firmware image specifies. Any code that attempts to access the (nonexistent) flash ROM is redirected to a `fakeflash` device, which implements the _bare minimum_ memory locations required to continue execution (mainly things related to model identification).

At this stage in development, it is _not_ required to have a Flash ROM image to run `clicky`, since any attempts at performing a LLE boot will most likely fail catastrophically (given the current number of stubbed out / nonexistent devices).

That said, if you're interested in helping out with `clicky`'s development, you might want to try to get your hands on a flash ROM image. If you happen to still have an old iPod lying around, you can dump the contents of it's flash ROM using Rockbox, as described [here](https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code).


## Roadmap

The plan is to implement devices and hardware "just in time" throughout development, instead of attempting to one-shot the entire SOC right off the bat. As such, the idea is to gradually test more and more complex software in the emulator, implementing more and more hardware as required.

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
- [ ] Get through the more complex https://github.com/iPodLinux/ipodloader2/
    - Touches even _more_ iPod-specific hardware (ATA-2)
    - Seems to do more in-depth system init (interrupt handling as well?)
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
- [Rockbox](https://www.rockbox.org/) - for additional iPod reverse engineering work
- [Sean Purcell](https://github.com/iburinoc/) - for writing the bulk of [armv4t_emu](https://github.com/daniel5151/armv4t_emu)
