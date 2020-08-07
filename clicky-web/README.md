# clicky-web

Run `clicky` on the web, using the power of WebAssembly!

**WARNING:** this port is _incredibly WIP!_

## Port Status

`clicky-web` is working, but runs _very slowly_. My guess is that `clicky-core` needs to be properly profiled + optimized.

Also, the HTML5/CSS/JavaScript is absolute spaghetti, and the Rust bindings aren't super clean either... They get the job done, but really aught to be rewritten [in Typescript].

## Controls

| iPod        | Keyboard | Mouse        | UI Element status |
| ----------- | -------- | ------------ | ----------------- |
| Menu        | Up       |              | 0%                |
| Reverse     | Left     |              | 0%                |
| Forward     | Right    |              | 0%                |
| Play/Pause  | Down     |              | 0%                |
| Select      | Enter    |              | 100%              |
| Click wheel |          | Scroll wheel | 25%\*             |
| Hold        | H        |              | 0%                |

\* only works on desktop (no touch support). Pretty glitchy.

## Dependencies

See https://rustwasm.github.io/book/game-of-life/setup.html for a list of programs and utilities to install.

Additionally, you'll need to copy a valid firmware and disk image to `clicky-web/www/resources/`, and `gzip` them. See the top-level `README.md` for details on building firmware / disk images.

## Building

Navigate to `clicky-web/` in your terminal, and run:

```bash
cargo watch -i .gitignore -i "pkg/*" -i "www/*" -i "../src/*" -s "wasm-pack build --release"
```

Navigate to `clicky-web/www/` in another terminal, and run:

```bash
npm install # just once
npm run start
```

Assuming everything went well, you should be able to access `click-web` at `localhost:8080`.
Open the Developer Console to see a whole bunch of debug logs :)
