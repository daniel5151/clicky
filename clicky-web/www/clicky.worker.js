// lol this is trash
let wasm = null;
import("clicky-web").then((x) => {
    wasm = x;
    // kick off the state machine
    postMessage({ kind: "ready" });
});

let ipod4g = null;
let ipod4g_controls = null;

function init_handler({ kind, data }) {
    switch (kind) {
        case "init":
            ipod4g = new wasm.Ipod4gContainer(data.bootloader, data.disk);
            ipod4g_controls = ipod4g.take_controls();
            console.log(ipod4g);
            console.log(ipod4g_controls);
            postMessage({ kind: "init" });
            return true;
            break;
        default:
            console.error("unknown init message sent to webworker: ", {
                kind,
                data,
            });
            postMessage({ kind: "unknown" });
            break;
    }

    return false;
}

function send_frame() {
    const frame = ipod4g.get_frame();
    const { width, height } = frame;
    const data = {
        width,
        height,
        data: frame.get_data(),
    };
    postMessage({
        kind: "frame",
        data,
    });
}

function run_handler({ kind, data }) {
    switch (kind) {
        case "frame":
            send_frame();
            break;
        case "keydown":
            console.log("pressed ", data);
            switch (data) {
                case "ArrowDown":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Down);
                    break;
                case "ArrowUp":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Up);
                    break;
                case "ArrowLeft":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Left);
                    break;
                case "ArrowRight":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Right);
                    break;
                case "Enter":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Action);
                    break;
                case "H":
                    ipod4g_controls.on_keydown(wasm.Ipod4gKeyKind.Hold);
                    break;
            }
            break;
        case "keyup":
            console.log("released ", data);
            switch (data) {
                case "ArrowDown":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Down);
                    break;
                case "ArrowUp":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Up);
                    break;
                case "ArrowLeft":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Left);
                    break;
                case "ArrowRight":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Right);
                    break;
                case "Enter":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Action);
                    break;
                case "H":
                    ipod4g_controls.on_keyup(wasm.Ipod4gKeyKind.Hold);
                    break;
            }
            break;
        case "scroll":
            if (data.deltaY < 0) {
                console.log("scolled up");
                ipod4g_controls.on_scroll(0, 2);
            } else {
                console.log("scolled down");
                ipod4g_controls.on_scroll(0, -2);
            }
            break;
        case "drive":
            ipod4g.run(1024); // tweak for different responsiveness
            postMessage({ kind: "drive" });
            break;
        default:
            console.error("unknown message sent to webworker", {
                kind,
                data,
            });
            postMessage({ kind: "unknown" });
            break;
    }

    return false;
}

const STATE_INIT = "INIT";
const STATE_RUNNING = "RUNNING";

let state = STATE_INIT;

onmessage = function (e) {
    if (wasm === null) {
        console.error(
            "wasm hasn't been loaded yet, why did you send me a message!",
        );
    }
    if (!e.data.kind || !e.data.data) {
        console.error("invalid format.");
        return;
    }

    switch (state) {
        case STATE_INIT:
            if (init_handler(e.data)) {
                state = STATE_RUNNING;
            }
            break;
        case STATE_RUNNING:
            {
                run_handler(e.data);
            }
            break;
    }
};
