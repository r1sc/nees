import { make_renderer } from "./renderer";
import { init, tick } from "../pkg/nees_wasm";
import wasminit from "../pkg/nees_wasm";

declare global {
    interface Window {
        waveout_callback: (sample: number) => void;
    }
}

window.onclick = async function () {
    window.onclick = null;

    const audio = new AudioContext({ sampleRate: 15720 });
    await audio.audioWorklet.addModule("nes-audio-processor.js");
    const audioNode = new AudioWorkletNode(audio, "nes-audio-processor", {
        channelCount: 1,
        channelCountMode: "explicit",
    });
    audioNode.connect(audio.destination);

    window.waveout_callback = (sample: number) => {
        audioNode.port.postMessage({ sample: sample });
    };


    await wasminit("nees_wasm_bg.wasm");


    const { gl, draw } = make_renderer();

    async function get_rom_from_url(url: string) {
        const response = await fetch(url);
        const buffer = await response.arrayBuffer();
        return new Uint8Array(buffer);
    }

    const rom = await get_rom_from_url("roms/smb3.nes");

    const nees_ptr = init(rom);
    const framebuffer = new Uint32Array(256 * 240);
    const fb_u8 = new Uint8Array(framebuffer.buffer);

    let last_time = performance.now();
    let accum = 0;
    let need_render = true;
    const target_ms = 1000 / 60;

    let player1_buttons_down = 0;
    let player2_buttons_down = 0;

    window.addEventListener("keydown", (e) => {
        if (e.key === "s") {
            player1_buttons_down |= 1 << 0;
        } else if (e.key === "a") {
            player1_buttons_down |= 1 << 1;
        } else if (e.key === "q") {
            player1_buttons_down |= 1 << 2;
        } else if (e.key === "w") {
            player1_buttons_down |= 1 << 3;
        } else if (e.key === "ArrowUp") {
            player1_buttons_down |= 1 << 4;
        } else if (e.key === "ArrowDown") {
            player1_buttons_down |= 1 << 5;
        } else if (e.key === "ArrowLeft") {
            player1_buttons_down |= 1 << 6;
        } else if (e.key === "ArrowRight") {
            player1_buttons_down |= 1 << 7;
        }

    });

    window.addEventListener("keyup", (e) => {
        if (e.key === "s") {
            player1_buttons_down &= ~(1 << 0);
        } else if (e.key === "a") {
            player1_buttons_down &= ~(1 << 1);
        } else if (e.key === "q") {
            player1_buttons_down &= ~(1 << 2);
        } else if (e.key === "w") {
            player1_buttons_down &= ~(1 << 3);
        } else if (e.key === "ArrowUp") {
            player1_buttons_down &= ~(1 << 4);
        } else if (e.key === "ArrowDown") {
            player1_buttons_down &= ~(1 << 5);
        } else if (e.key === "ArrowLeft") {
            player1_buttons_down &= ~(1 << 6);
        } else if (e.key === "ArrowRight") {
            player1_buttons_down &= ~(1 << 7);
        }
    });

    (function render() {
        const now = performance.now();
        let delta = now - last_time;
        last_time = now;

        if(delta > 500) {
            delta = 500;
        }

        accum += delta;

        while (accum >= target_ms) {
            tick(nees_ptr, framebuffer, player1_buttons_down, player2_buttons_down);
            accum -= target_ms;
            need_render = true;
        }

        if (need_render) {
            draw(fb_u8);
            need_render = false;
        }


        requestAnimationFrame(render);
    })();

};