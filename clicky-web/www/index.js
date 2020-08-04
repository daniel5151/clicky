import * as wasm from "clicky-web";

import 'modern-css-reset';
import './main.css';

const canvas = document.getElementById('ipod-screen');
const ctx = canvas.getContext('2d');

wasm.draw(ctx, 160, 128);
