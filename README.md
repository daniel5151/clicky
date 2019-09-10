# clicky 

A classic clickwheel iPod emulator.

## Usage

If only! No, no... we are still in the _very_ early stages here haha.

## The Gameplan

MVP target hardware: the iPod 4g (Grayscale) (PP5020).

- [ ] Get through the basic https://github.com/iPodLinux/ipodloader/
    - This touches quite a bit of iPod-specific hardware (e.g: Timers, Buttons, LCD)
    - Short enough that I can step though the code instruction-by-instruction
    - **Goals:**
        - Find my footing with ARM7TDMI, and the iPod's funky dual-processor architecture
        - Get more familiar with the arm7tdmi assembler & compiler toolchain
        - Set up project boilerplate
            - LCD output, button input
            - File IO
        - Scaffold some basic system architecture (CPU + MMU + some simple devices)
- [ ] Get through the more complex https://github.com/iPodLinux/ipodloader2/
    - Touches even _more_ iPod-specific hardware (including the _Color_ LCD)
    - Seems to do more in-depth system init (interrupt handling as well?)
    - **Goals:**
        - Expand on the system architecture + implemented devices
- [ ] Boot into [Rockbox](https://www.rockbox.org/)
    - A gargantuan task, that will involve implementing a _lot_ of misc. hardware
    - Since the OS is open source, is _should_ be possible to trace through the code, and infer what the hardware is supposed to do.
    - **Goals:**
        - Boot an actual OS on the iPod
- [ ] Boot into [iPod Linux](http://www.ipodlinux.org/)
    - A bigger beast than Rockbox, and likely much more difficult to step through & debug
    - **Goals:**
        - Booth _another_ actual OS on the iPod
        - Fill in the gaps between the hardware Rockbox uses, and the hardware iPod Linux uses 
- [ ] Boot into RetailOS
    - i.e: _the big money goal_
    - Hopefully, by getting two other OSs up and running, RetailOS will "just work"
    - Realistically, those Apple engineers probably did some fancy/janky stuff, and things will be very broken
    - **Goals:**
        - Get an actual working emulated iPod up and running!
        - Play some Brick Breaker!

## Unknowns that I'm scared of

- Funky cache effects
    - I _really_ don't want to deal with implementing proper caching if I don't have to. I'm gonna cross my fingers, and hope that having both CPUs see memory writes at the same time will be _fiiiiine_
- ARM emulator issues
    - Boy, it sure would _suck_ if the CPU emulator had bugs in it!
    - Boy, it sure would _suck_ if the iPod actually made use of ARM's configurable Endianness!
- Funky iPod hardware that _hasn't_ been reverse engineered
    - ...this will suck, and unfortunately, It's probably something I'll encounter once I start messing around with RetailOS.


## Things I won't be tackling off the bat

- Audio
    - inb4 "but it's an iPod, it's literally an _audio player_"
    - yeah, I know, but Audio is hard, and finicky to get right, so I'll be leaving it for _waaaaaay_ later

## Things I _might_ look into

- Assembler & C source-code maps (for easier Debugging)
    - This would likely be implemented as part of [arm7tdmi-rs](https://github.com/daniel5151/arm7tdmi-rs), as it isn't something iPod specific.

## Long-term plans

The initial inspiration for this project was discovering that no one has ever looked into preserving those old [iPod Games](https://en.wikipedia.org/wiki/IPod_game). And yeah, they're not _great_, but still, wouldn't it be near to be able to play them on modern hardware?

Those games are compatible with the iPod 5g and up. I have a sneaking suspicion that they might make use of some funky additional hardware (\*cough\* the _very closed source_ Broadcom BCM2722 video chip \*cough\*), which might makes things a bit tricky...

## Acknowledgments

- [Sean Purcell](https://github.com/iburinoc/) - for writing [arm7tdmi-rs](https://github.com/daniel5151/arm7tdmi-rs)
- [iPod Linux](http://www.ipodlinux.org/) - for invaluable iPod reverse engineering work
- [Rockbox](https://www.rockbox.org/) - for additional iPod reverse engineering work
