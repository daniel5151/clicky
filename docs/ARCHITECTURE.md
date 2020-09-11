# `clicky`'s High Level Architecture

**NOTE:** This document primarily discusses `clicky-core`'s architecture, as at the time of writing, the various frontend crates don't really have any "interesting" code in them.

**TODO:** This document could use more detail, and/or the inline rustdoc comments for each module should be more detailed.

## Preface: Using `cargo doc` to Explore `clicky-core`

External documentation is great, but it has a tendency of drifting from the reality of the source code over time.

This document is a best-effort attempt to document `clicky`'s broader architecture. This is a _descriptive_ document, not a _prescriptive_ document, and as such, it may drift from the reality of the source code over time. When in doubt, follow the code!

Thankfully, Rust comes with an excellent built-in documentation tool: `cargo doc`! Most types, traits, methods, and modules have an associated doc-comment which provide a brief overview of what the item does.

This approach is excellent at giving a broad overview of `clicky`'s broad layout, but doesn't provide too much insight into how things are implemented "under the hood." As such, it's _highly_ recommended to skim through the code itself (which also includes plenty of non-doc comments) to get an understanding of how `clicky` works.

## Separation between `clicky-core` and it's various frontends

The `clicky-core` crate contains the core emulation logic, and is designed to be _platform agnostic_. If a device needs to read a file, get user input, draw something to the screen, etc... it must expose a public interface that can delegate the details of the operation to an external "front-end".

For example, the `display/hd66753` device emulates the iPod 4g's LCD display, but instead of including platform-specific rendering logic in the device, it simply provides a `render_callback()` method, which returns a callback that can be used to fetch a RGBA framebuffer. Later, in the frontend code, this callback can be wired up to whatever rendering pipeline that's required, e.g: a `canvas` context in `clicky-web`.

This separation keeps the core emulation code portable, and should make it possible to run `clicky` on devices ranging from typical desktop PCs, to web browsers, and (hopefully) mobile-phones.

## High-Level Source Organization

The following is a lightly-annotated overview of `clicky-core`'s source structure:

```
clicky-core
├── Cargo.toml
└── src
    ├── lib.rs ................... Re-exports of subsequent modules
    │  
    ├── error.rs ................. Various Error types + error handling logic
    │  
    ├── block .................... Block-device abstraction
    │   ├── mod.rs ................. Core block-device traits
    │   └── backend ................ Concrete block-device implementations
    │       ├── mem.rs ............... e.g: Memory backed
    │       ├── raw.rs ............... e.g: Raw File backed
    │       └── ...
    │  
    ├── devices .................. Peripheral Devices
    │   ├── mod.rs ................. Core peripheral-device traits (i.e: the `Device` trait)
    │   ├── prelude.rs ............. Common imports to simplify implementing devices
    │   │
    │   ├── display ................ Display devices (i.e: those that expose a framebuffer)
    │   ├── generic ................ Platform-agnostic devices
    │   │   ├── asanram.rs ........... e.g: RAM with additional address-sanitation instrumentation
    │   │   ├── ide .................. e.g: Generic IDE controller
    │   │   └── ...
    │   ├── i2c .................... I2C Devices (use separate i2c bus interface)
    │   │   ├── prelude.rs ........... Category-specific device prelude
    │   │   └── devices
    │   │       └── ...
    │   ├── platform ............... Platform-specific devices
    │   │   ├── pp.rs ................ e.g: PortalPlayer/iPod-specific devices
    │   │   ├── pp
    │   │   │   ├── memcon.rs .......... e.g: Memory Controller
    │   │   │   ├── piezo.rs ........... e.g: Piezo speaker
    │   │   │   ├── ppcon.rs ........... e.g: PortalPlayer internal controller
    │   │   │   └── ...
    │   │   └── ..
    │   └── util ................... Utility devices (typically wrappers around other devices)
    │  
    ├── executor ................. Custom `async/await` executor
    │  
    ├── gui ...................... Types / Traits for exposing GUI interfaces
    │  
    ├── memory ................... Core `Memory` interface + helper types
    │  
    ├── signal ................... Cross-device signaling mechanism
    │   ├── gpio.rs ................ e.g: Signal GPIO pin change
    │   ├── irq.rs ................. e.g: Assert/clear IRQ lines
    │   └── mod.rs
    │  
    └── sys ...................... Top-level System Definitions
        ├── ipod4g ................. e.g: Defines the `iPod 4g` system
        │   ├── controls.rs .......... System-specific user input structures
        │   ├── gdb.rs ............... GDB stub
        │   ├── hle_bootloader ........HLE bootloader implementation
        │   └── mod.rs ............... Core implementation
        └── ...
```

If you've ever had some experience with emulator development, this structure should be somewhat familiar (at least in the broad strokes). Additionally, if you've ever worked on something like QEMU, this structure should be _very_ familiar to you.

One notable omission is the lack of any in-tree CPU emulation. At the moment, `clicky-core` uses the `armv4t_emu` crate from `crate.io` for it's CPU emulation. It's proven to be a fairly robust implementation of a ARMv4T interpreter, though it would be nice to switch to a JIT at some point in the future.
