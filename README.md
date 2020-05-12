# clicky

A _very WIP_ clickwheel iPod emulator.

**This project is not ready for end users,** but is _very_ open to contributors!

## Building `clicky`

`clicky` uses the standard `cargo` build flow, with one small caveat: `clicky` currently requires having a local copy of the `arm7tdmi-rs` crate in it's parent directory.

```bash
# arm7tdmi-rs must be cloned locally
git clone https://github.com/daniel5151/arm7tdmi-rs.git
cd arm7tdmi-rs
git checkout dev # and it must be on the dev branch
cd ..
git clone https://github.com/daniel5151/clicky.git
cd clicky
cargo build
```

## Building a test firmware

At this stage in development, `clicky` can't run anything too complex, so a good jumping off point is building a firmware image based off `ipodloader`.

I've included `ipodloader`'s source code in-tree under `./resources/ipodloader`. It's Makefile has been modified to work on modern linux distros, and I've toggled some compiler flags to disable optimizations and enable debug symbols (which should make stepping through the code a lot easier).

**NOTE:** Building `ipodloader` requires installing the gcc `arm-none-eabi` toolchain.
On Ubuntu / Debian based distros: `sudo apt install gcc-arm-none-eabi`

```bash
cd ./resources/ipodloader
printf "\xDE\xAD\xBE\xEF" > deadbeef.bin # create a dummy payload
make
./make_fw -v -g 4g -o ipodloader_deadbeefs_unopt.bin -l deadbeef.bin -l deadbeef.bin loader.bin
```

In the future, `clicky` should be able to run more complex images (such as Rockbox, iPodLinux, or even Apple's own RetailOS), but that's a ways off.

## Running `clicky`

Now that you have an iPod firmware image, you can finally run clicky:

```bash
RUST_LOG=trace cargo run ./resources/ipodloader/ipodloader_deadbeefs_unopt.bin hle ./resources/ipodloader/loader.objdump
```

It's not pretty, but if you press enter a couple time, you should be able to step through some CPU instructions. My hacky objdump-based addr2line implementation will even show you the `ipodloader` source code associated with the current PC!

Typing 'r' and hitting enter will run the CPU until it hits a breakpoint / crashes.
Breakpoints are currently hard-coded into the source code.

### (OPTIONAL) Using `clicky` with an Apple flash ROM dump

A proper Low Level Emulation (LLE) boot process would involve booting the CPU from address 0 and having it execute whatever bootloader code is present in Flash ROM. This code most likely performs several different functions, including setting up devices, toggling certain interrupts, and of course, loading the actual firmware image from the emulated HDD into executable memory.

The code contained in Flash ROM is copyrighted by Apple, and as such, `clicky` cannot legally redistribute copies of it. Instead, `clicky` currently uses a High Level Emulation (HLE) approach to boot firmware images, where `clicky` "fakes" the real bootleader by loading the firmware image into memory, and starting execution from whatever address the firmware image specifies. Once the CPU is running, any code that accesses the (nonexistent) flash ROM is redirected to a `fakeflash` device, which implements the _bare minimum_ amount of code required to continue execution.

At this stage in development, it is _not_ required to have a Flash ROM image to run `clicky`, since any attempts at performing a LLE boot will most likely fail catastrophically (given the plethora of missing devices).

That said, if you're interested in helping out with `clicky`'s development, you might want to try to get your hands on a flash ROM image. If you happen to still have an old iPod lying around, you can dump the contents of it's flash ROM using Rockbox, as described [here](https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code).

## Contributors Welcome!

If you're interested in helping preserve a piece of iconic hardware from the early 2000s, you're more than welcome to help out!

### Diving in

For concrete things that need fixing, check out the issue tracker.

Alternatively, if you're a more "gung-ho" kinda person, and want to get your hands dirty, your best bet would be to load up a firmware image, hit run, and implement whatever unimplemented hardware you come across!

E.g: Say you run `clicky`, and it crashes with something like this:

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

If you're lucky, and the crash happens somewhere with debug symbols, you might even get the line of code that performed the invalid access!

```
0x40000a58:  ldr r2, [r3]
.../clicky/resources/ipodloader/tools.c:125
<lcd_wait_write+0x10>
 /* wait for LCD with timeout */
 void
 lcd_wait_write()
 {
>   if ((inl(lcd_base) & lcd_busy_mask) != 0) {
        int start = timer_get_current();

        do {
            if ((inl(lcd_base) & lcd_busy_mask) == 0) break;
```

This should give you the info you need to start augmenting / implementing the associated device. Take a peek at how other devices are implemented in the repo for an idea of how to structure your new device.

That said, `clicky`'s core architecture is still very-much in flux, with the current traits, structs, and overall code-layout likely to change and evolve as more devices get implemented, and more requirements are uncovered.

### Resources

Documentation on the iPod can be pretty scarce, so I've collected a handful of helpful resources and documentation under the `./resources/` folder. The README.md has links to additional online resources. If you happen to stumble across a resource that you think is helpful to the project don't hesitate to add it to the folder!

## The Development Gameplan

### Target Hardware

- MVP: iPod 4g (Grayscale)
- End goal: iPod 5g

Why these models?

The 4g uses the same/similar SOC as some of the later generation models (PP5020), while also using simpler display. _Hopefully_, this will translate into less time spent on display emulation, and more time spent on getting other devices and misc hardware working.

The 5g is the first iPod model to support those aforementioned [iPod Games](https://en.wikipedia.org/wiki/IPod_game), so getting it up and running would be super cool.

### Development Roadmap

The Rockbox source code is proving incredibly useful in getting things up and running. [pp5020.h](https://github.com/Rockbox/rockbox/blob/master/firmware/export/pp5020.h) and [ipod4g.h](https://github.com/Rockbox/rockbox/blob/master/firmware/export/config/ipod4g.h) are invaluable in providing a high level overview of the hardware, and by grepping the codebase for specific defines, it's easy to find code that describes how the hardware is supposed to work.

Devices and hardware will be implemented "just in time" as the software tries to access them (instead of attempting to one-shot the entire SOC right off the bat). As such, the idea will be to gradually test more and more complex software in the emulator as more hardware is implemented.

- [ ] Execute something _really_ basic, such as https://github.com/iPodLinux/ipodloader/
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
    - Touches even _more_ iPod-specific hardware
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

### Things that might be worth looking into

- Assembler & C source maps (for Debugging)
    - This would likely be implemented as part of [arm7tdmi-rs](https://github.com/daniel5151/arm7tdmi-rs), as it isn't something iPod specific.
    - **update:** I've thrown _something_ together (see `src/debugger/asm2line.rs`) but it really should be rewritten / totally thrown out. it's _really_ bad code. A better approach would be to write a gdb stub.

---

## Fluff: Why emulate the iPod?

I enjoy a good technical challenge, plain and simple!

Compared to my last emulation project ([ANESE](https://prilik.com/ANESE), a NES emulator that [automatically maps out NES games](https://prilik.com/blog/wideNES)), the iPod presents a totally different set of technical challenges to overcome.

First of all, the iPod is a fairly modern system. Unlike the esoteric and custom-made chips used in many game consoles, the iPod uses many off-the-shelf commodity hardware and technologies. As such, this project should be a good way to explore and learn more about the low level details of tech such as ARM assembly, I2S, I2C, USB, IDE HDDs, etc...

Second of all, the iPod isn't very well documented! While this is probably going to be annoying in the long run, I'm excited to do my own research, discover new information, and consolidate information on the iPod myself (as opposed to already having a well organized and complete reference at my disposal \*cough\* the nesdev wiki \*cough\*). As it turns out, there's already quite a amount of documentation about the iPod that's floating around (thanks to the iPodLinux and RockBox projects), but I'm sure there will still be plenty of stuff left for me to discover.

Lastly, the iPod is a system that's never been emulated before! That means there usually won't be any sort of "escape hatch" when I get stuck, since there's no one else's code I can peek at. Whatever challenges I run in to will be challenges I'm going to have to solve myself! How exciting!

...there is one last reason I want to emulate the iPod though:

**It's got _Brick Breaker!_**

> _ooooooh Brick Breaker baybeeeeee! This game has won game of the year, I don't know how many times!_

But seriously, aside from brick breaker, there were actually a whole bunch of [iPod Games](https://en.wikipedia.org/wiki/IPod_game) released for late-gen iPod models \~2006. While these games aren't necessarily _masterpieces_, they're still pretty neat, and aught to be preserved.

in fact, my initial inspiration for starting this project was actually hearing about these old games, and how no one has ever looked into preserving them. While getting these games working will probably take quite a while, it's a neat long-term goal to aim for.

## Thanks and Acknowledgments

This project would be dead in the waters without these folks and projects:

- [Sean Purcell](https://github.com/iburinoc/) - for writing [arm7tdmi-rs](https://github.com/daniel5151/arm7tdmi-rs)
- [iPod Linux](http://www.ipodlinux.org/) - for invaluable iPod reverse engineering work
- [Rockbox](https://www.rockbox.org/) - for additional iPod reverse engineering work
