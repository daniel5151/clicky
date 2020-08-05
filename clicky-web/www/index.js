import * as wasm from "clicky-web";

import 'modern-css-reset';
import './main.css';

const canvas = document.getElementById('ipod-screen');
const ctx = canvas.getContext('2d');

console.log(wasm)

async function get_uint8_array(url) {
    let res = await fetch(url);
    let blob = await res.blob();
    let data = new Uint8Array(await blob.arrayBuffer());
    return data
}

async function run_clicky_ipod4g() {
    console.log("loading bootloader...");
    let bootloader = await get_uint8_array("./resources/bootloader_with_rockbox_dma.bin.gz");
    console.log("loading disk image...");
    let disk = await get_uint8_array("./resources/ipodhd_rockbox_stock.img.gz");

    let clicky_controller = new wasm.Ipod4gContainer(bootloader, disk);
}

run_clicky_ipod4g()
