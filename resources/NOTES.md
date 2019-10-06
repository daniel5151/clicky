# Notes

A grab-bag of facts / thoughts I discover.

---

As it turns out, calling the iPodLinux / Rockbox bootloaders "bootloaders" is a little bit misleading, as they aren't actually the code which runs immediately after power-on!

As it turns out, execution starts in Flash memory (mapped from 0x0), and the first real code that gets run is the stock Apple bootloader. It's _that_ code which handles reading the firmware _from Disk_, parsing it's contents, and loading an image into memory.

As far as I can tell, iPodLinux / Rockbox "cheat" and don't overwrite the stock Apple bootloader. Instead, they craft firmware images where the OS image actually points to a custom second-stage bootloader. Thus, once the stock loader dutifully loads what it thinks is the OS, the second-stage bootloader runs, and loads the _actual_ OS.

Alright, no problem, right? Let's just slap a copy of an iPod's flash ROM at 0x0, run the CPU, and let it run it's course, easy!

Alas, it's not that easy, and as far as I can tell, no one ever dumped an iPod's flash memory directly.

So, what's the plan of attack then?

I'll simulate what I _think_ the stock apple bootloader does directly in my emulator's code (i.e: parse the firmware file according to [it's spec](http://www.ipodlinux.org/Firmware.html), load the image I want into memory, and start the CPU from there directly).

This ain't _great_, since I'll have to make some assumptions about the system's state (since it won't be a true "cold-start"), and hope that the stock bootloader didn't poke / init _too_ much hardware...

... now, here's the sneaky bit:

Instead of loading the OS image (with the second-stage bootloader), what if I instead loaded an "aupd" image from a legit apple firmware image? That code _should_ then update the flash ROM with a new bootloader image, whereupon I could dump the contents of my emulator's memory, and then do a cold-start with the extracted code!

This would be awesome, but likely be quite difficult to pull off properly, as I would have to reverse-engineer how the Flash ROM is written to, and implement the actual Flash ROM hardware to get it working correctly.

As such, I'll likely begin by using my high-level bootloader to load the second-stage bootloader, just to get things going. Once I feel more comfortable with this whole endeavor, I'll revisit this idea...

---

Oh shit would you look at that.

https://www.rockbox.org/wiki/IpodFlash#Apple_39s_flash_code

Looks like the rockbox devs also wanted to pull the contents of flash ROM from iPods, and wrote a utlity to do so.

That's great, but unfortunately for me, _I don't have an iPod to rip the flash from!_. Guess I'll just have to keep chugging along with the HLE bootloader approach...

---

Guess who finally got an iPod 4g and managed to rip the flash ROM from it??? THIS GUY!

It's definately nice to have, though now that I have it, I realize that it still makes sense to HLE boot (bypassing the flash ROM bootloader), as I _really_ don't want to start working on HDD emulation. On the bright side, no more guessing the contents of flash ROM on a byte-by-byte basis!
