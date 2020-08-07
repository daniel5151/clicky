# clicky-desktop

A native CLI + GUI for `clicky`.

## Controls

| iPod        | `clicky-desktop` |
| ----------- | ---------------- |
| Menu        | Up               |
| Reverse     | Left             |
| Forward     | Right            |
| Play/Pause  | Down             |
| Select      | Enter            |
| Click wheel | Scroll wheel     |
| Hold        | H                |

## Building

Building `clicky-desktop` is quite straightforward, and uses the standard `cargo` build flow:

```bash
# from the top-level `clicky/` workspace
cargo build --release -p clicky-desktop
```

**Warning:** Building without `--release` will be very slow!

#### Common build errors

-   Due to a `cargo` limitation ([rust-lang/cargo#5364]), toggling feature flags within sub-packages of a workspace is a bit clunky.
    -   At the moment, there's only a single feature (`minifb`), so this shouldn't be a problem.
-   (Linux) You may encounter some build-script / linker errors related to missing `xkbcommon` and `wayland` libraries. On Debian/Ubuntu, you can install them via `apt install libxkbcommon-dev libwayland-dev`.

## Running

`clicky-desktop` requires various files to run:

-   A HDD image
-   A firmware binary (when using HLE)
-   (optional) A Flash ROM dump (when using LLE - see `DEVGUIDE.md`)

See the top-level `README.md` for details on obtaining / creating these files.

### Examples

-   Basic end-user
    -   HLE bootloader
    -   `--hdd=file` indicates HDD writes are written directly back to the disk image

```bash
cargo run -p clicky-desktop --release -- --hdd=file:file=/path/to/ipodhd.img --hle=/path/to/rockbox_fw.bin
```

-   Typical dev-cycle
    -   Use HLE bootloader alongside a real flash-rom dump
    -   `--hdd=mem` indicates that the image should be loaded into memory, and _not_ written back to disk. This is useful for ensuring reproducible runs.
    -   The `RUST_LOG` environment variable is used to tweak log levels for various emulator subsystems.
    -   Spawn a GDB server if a fatal error occurs.
    -   Run the GDB server over a Unix Domain Sockets (/tmp/clicky).
        -   Connect to the server using the GDB command `target remote /tmp/clicky`.

```bash
RUST_LOG=MMIO=info,GPIO=trace,gdbstub=error             \
cargo run -p clicky-desktop --release --                \
    --flash-rom=/path/to/internal_rom_000000-0FFFFF.bin \
    --hdd=mem:file=/path/to/ipodhd.img                  \
    --hle=/path/to/rockbox_bootloader_fw.bin            \
    -g /tmp/clicky,on-fatal-err
```
