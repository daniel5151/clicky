import "modern-css-reset";
import "./main.css";

import ClickyWorker from "./clicky.worker.js";

const canvas = document.getElementById("ipod-screen");
const ctx = canvas.getContext("2d");

async function get_uint8_array(url) {
    const res = await fetch(url);
    const blob = await res.blob();
    const data = new Uint8Array(await blob.arrayBuffer());
    return data;
}

// it's a ping-pong state machine, wee woo wee woo!

console.log("loading webworker...");
const worker = new ClickyWorker();
worker.onmessage = (e) => {
    const { kind, data } = e.data;
    switch (kind) {
        case "ready":
            // run init logic
            console.log("loaded webworker!");

            (async () => {
                console.log("loading bootloader...");
                // let bootloader = await get_uint8_array("./resources/ipodloader_loops_unopt.bin.gz");
                let bootloader = await get_uint8_array(
                    "./resources/bootloader_with_rockbox_dma.bin.gz",
                );
                console.log("loading disk image...");
                let disk = await get_uint8_array(
                    "./resources/ipodhd_rockbox_stock.img.gz",
                );

                worker.postMessage({
                    kind: "init",
                    data: { bootloader, disk },
                });
            })();
            break;
        case "init":
            // attach all the event handlers
            window.onkeydown = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keydown",
                    data: e.key,
                });
            };

            window.onkeyup = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keyup",
                    data: e.key,
                });
            };

            document.querySelector("body").addEventListener(
                "mousewheel",
                (e) => {
                    e.preventDefault();
                    worker.postMessage({
                        kind: "scroll",
                        data: {
                            deltaX: e.deltaX,
                            deltaY: e.deltaY,
                        },
                    });
                },
                { passive: false },
            );

            // boot up the renderer
            worker.postMessage({
                kind: "frame",
                data: {},
            });

            // and the driver
            worker.postMessage({
                kind: "drive",
                data: {},
            });
            break;
        case "frame":
            ctx.putImageData(
                new ImageData(
                    new Uint8ClampedArray(data.data),
                    data.width,
                    data.height,
                ),
                0,
                0,
            );

            window.requestAnimationFrame(() => {
                // setTimeout(() => {
                worker.postMessage({
                    kind: "frame",
                    data: {},
                });
                // }, 1000 / 60);
            });
            break;
        case "drive":
            worker.postMessage({
                kind: "drive",
                data: {},
            });
            break;
        default: {
            console.error("unknown message from worker: ", e.data);
        }
    }
};
