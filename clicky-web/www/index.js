import "modern-css-reset";
import "./main.css";

import ClickyWorker from "./clicky.worker.js";

const fps_counter = document.getElementById("fps");
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
window.clicky_worker = worker; // for debugging
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
            const $ = (...args) => document.querySelector(...args);

            $("#ipod-container").onkeydown = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keydown",
                    data: e.key,
                });
            };

            $("#ipod-container").onkeyup = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keyup",
                    data: e.key,
                });
            };

            $("#ipod-btn-select").onmousedown = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keydown",
                    data: "Enter",
                });
            };
            $("#ipod-btn-select").onmouseup = (e) => {
                e.preventDefault();
                worker.postMessage({
                    kind: "keyup",
                    data: "Enter",
                });
            };

            let wheel_down = false;
            $("#ipod-clickwheel").onmousedown = (e) => {
                e.preventDefault();
                wheel_down = true;
            };
            $("#ipod-clickwheel").onmouseup = (e) => {
                e.preventDefault();
                wheel_down = false;
            };
            $("#ipod-clickwheel").onmouseleave = (e) => {
                e.preventDefault();
                wheel_down = false;
            };

            // needs a _lot_ of tweaking
            let last_angle = 0;
            $("#ipod-clickwheel").onmousemove = (e) => {
                e.preventDefault();
                if (wheel_down) {
                    const new_angle =
                        Math.atan2(e.offsetX - 80 * 2, -(e.offsetY - 80 * 2)) *
                            (180 / Math.PI) +
                        180;

                    if (Math.abs(last_angle - new_angle) > 360 / 36) {
                        worker.postMessage({
                            kind: "scroll",
                            data: {
                                deltaX: 0,
                                deltaY: new_angle - last_angle,
                            },
                        });
                        last_angle = new_angle;
                    }
                }
            };

            $("#cycles").onchange = (e) => {
                worker.postMessage({
                    kind: "cycles_per_tick",
                    // haha, arbitrary code execution go brrrrr
                    data: eval(e.target.value),
                });
            };

            $("body").addEventListener(
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
            const now = performance.now();
            fps_counter.innerHTML = Math.floor(
                1 / ((now - window.last_frame) / 1000),
            );
            window.last_frame = now;

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
