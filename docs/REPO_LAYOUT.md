# Repo Layout

This document describes the general layout of the `clicky` project.

## Crates

At the top level, `clicky` is split up into multiple crates:

| crate            | type |                                                                                  |
| ---------------- | ---- | -------------------------------------------------------------------------------- |
| `clicky-core`    | lib  | Platform agnostic emulator code.                                                 |
| `clicky-desktop` | bin  | A native CLI + GUI to interact with `clicky-core`.                               |
| `clicky-web`     | bin  | Run `clicky` on the web using the power of `wasm`! (_very_ WIP)                  |
| `relativity`     | lib  | Cross-platform timers and `Instant` which can be paused/resumed/shifted in time. |

These crates live in a single [`cargo` workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html), defined in the top-level `Cargo.toml`.

### `clicky-desktop` and `clicky-web`

At the moment, `clicky` comes with two different frontends: `clicky-desktop` and `clicky-web`. As the names imply, these crates implement the platform-specific code required to run the GUI, load/save files, set up a GDB server, etc... for native clients, and web clients.

These crates are primarily comprised of easy-to-follow "glue" code, which simply connects the interfaces exposed by `clicky-core` to the outside world.

See the `README.md` files in the `clicky-desktop` and `clicky-web` directories for more details on how to build and work with the various frontends.

### `clicky-core`

This crate contains the core, platform-agnostic emulation code. It doesn't perform any I/O itself, and must be plugged into a frontend (such as `clicky-desktop`) to function.

### `relativity`

`relativity` is a library which provides cross-platform (read: native + wasm) timers and `Instant`s which can be paused/resumed/shifted in time. See it's `README.md` for more details.

It doesn't depend on any code from any of the `click-X` crates, and could theoretically be split off into it's own repo entirely.

## Documentation + Resources

Aside from code, the `clicky` repo includes various bits of documentation and resources to aid in development.

### `docs`

Hey, that's where this file lives!

As the name implies, this folder is where all `clicky`-specific documentation is collected. Things like how to compile/build `clicky`, how to build some basic software, `clicky`'s project structure and architecture, etc...

### `resources`

This folder contains various tidbits of iPod-related documentation and homebrew software which are useful references when working on / testing `clicky`.

If you stumble across any resources you think might be helpful to preserve "in-tree", feel free to add them here! Just make sure they can be legally distributed!

### `scripts`

This folder is a grab-bag of scripts related to `clicky` that automate certain aspects of development.

At the time of writing, these scripts are primarily used to create / format / update raw disk image files.

### `screenshots`

Contains screenshots of iPod software referenced in the `README.md`.
