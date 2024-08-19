import { make_renderer } from "./renderer";
import wasminit, { draw_osd, get_framebuffer_ptr, init, step_osd, StepResponse, tick } from "../pkg/nees_wasm";
import wasm_path from "../pkg/nees_wasm_bg.wasm";

// Get rom path from query string
const rom_path = new URLSearchParams(window.location.search).get("rom");
if (rom_path === null) {
    alert("No rom path provided in query string");
    throw new Error("No rom path provided in query string");
}

declare global {
    interface Window {
        waveout_callback: (sample: number) => void;
    }
}

const start_btn = document.createElement("button");
start_btn.textContent = "Click to start";
document.body.appendChild(start_btn);

start_btn.onclick = async function () {
    start_btn.remove();

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

    async function loadwasm() {
        const response = fetch(wasm_path as unknown as string);
        return await wasminit(response);
    }

    const o = await loadwasm();

    const { gl, draw } = make_renderer();

    async function get_rom_from_url(url: string) {
        const response = await fetch(url);
        const buffer = await response.arrayBuffer();
        return new Uint8Array(buffer);
    }

    const rom = await get_rom_from_url(rom_path);

    const nees_state_ptr = init(rom);

    const fb_ptr = get_framebuffer_ptr();
    const fb_u8 = new Uint8Array(o.memory.buffer, fb_ptr, 256 * 240 * 4);
    // const framebuffer = new Uint32Array(256 * 240);
    // const fb_u8 = new Uint8Array(framebuffer.buffer);

    let last_time = performance.now();
    let accum = 0;
    let need_render = true;
    const target_ms = 1000 / 60;

    let player_buttons: [number, number] = [0, 0];
    let b_button = ["k", "a"];
    let a_button = ["l", "s"];
    let select_button = ["i", "q"];
    let start_button = ["o", "w"];
    let up_button = ["ArrowUp", "t"];
    let down_button = ["ArrowDown", "g"];
    let left_button = ["ArrowLeft", "f"];
    let right_button = ["ArrowRight", "h"];

    let osd_enabled = false;

    window.addEventListener("keydown", (e) => {
        for (let i = 0; i < 2; i++) {
            if (e.key === a_button[i]) {
                player_buttons[i] |= 1 << 0;
            } else if (e.key === b_button[i]) {
                player_buttons[i] |= 1 << 1;
            } else if (e.key === select_button[i]) {
                player_buttons[i] |= 1 << 2;
            } else if (e.key === start_button[i]) {
                player_buttons[i] |= 1 << 3;
            } else if (e.key === up_button[i]) {
                player_buttons[i] |= 1 << 4;
            } else if (e.key === down_button[i]) {
                player_buttons[i] |= 1 << 5;
            } else if (e.key === left_button[i]) {
                player_buttons[i] |= 1 << 6;
            } else if (e.key === right_button[i]) {
                player_buttons[i] |= 1 << 7;
            }
        }

        if (e.key == "Escape") {
            e.preventDefault();
            osd_enabled = !osd_enabled;
 
            if (osd_enabled) {
                draw_osd(nees_state_ptr, fb_ptr);
            }
        } else if (osd_enabled) {
            let response: StepResponse | null = null;
            if (e.key == "ArrowUp") response = step_osd(nees_state_ptr, 0);
            else if (e.key == "ArrowDown") response = step_osd(nees_state_ptr, 1);
            else response = step_osd(nees_state_ptr, 2);

            if (response.action === 1) b_button[response.which_player] = e.key;
            else if (response.action === 2) a_button[response.which_player] = e.key;
            else if (response.action === 3) select_button[response.which_player] = e.key;
            else if (response.action === 4) start_button[response.which_player] = e.key;
            else if (response.action === 5) up_button[response.which_player] = e.key;
            else if (response.action === 6) down_button[response.which_player] = e.key;
            else if (response.action === 7) left_button[response.which_player] = e.key;
            else if (response.action === 8) right_button[response.which_player] = e.key;

            draw_osd(nees_state_ptr, fb_ptr);
        }
    });

    window.addEventListener("keyup", (e) => {
        for (let i = 0; i < 2; i++) {
            if (e.key === a_button[i]) {
                player_buttons[i] &= ~(1 << 0);
            } else if (e.key === b_button[i]) {
                player_buttons[i] &= ~(1 << 1);
            } else if (e.key === select_button[i]) {
                player_buttons[i] &= ~(1 << 2);
            } else if (e.key === start_button[i]) {
                player_buttons[i] &= ~(1 << 3);
            } else if (e.key === up_button[i]) {
                player_buttons[i] &= ~(1 << 4);
            } else if (e.key === down_button[i]) {
                player_buttons[i] &= ~(1 << 5);
            } else if (e.key === left_button[i]) {
                player_buttons[i] &= ~(1 << 6);
            } else if (e.key === right_button[i]) {
                player_buttons[i] &= ~(1 << 7);
            }
        }
    });

    (function render() {

        const now = performance.now();
        let delta = now - last_time;
        last_time = now;

        if (delta > 500) {
            delta = 500;
        }

        const [gp1, gp2] = navigator.getGamepads();
        function set_buttons_down(gp: Gamepad, player_index: number) {
            player_buttons[player_index] = 0;
            if (gp.buttons[0].pressed) {
                player_buttons[player_index] |= 1 << 0;
            }
            if (gp.buttons[2].pressed) {
                player_buttons[player_index] |= 1 << 1;
            }
            if (gp.buttons[8].pressed) {
                player_buttons[player_index] |= 1 << 2;
            }
            if (gp.buttons[9].pressed) {
                player_buttons[player_index] |= 1 << 3;
            }
            if (gp.axes[1] < -0.5 || gp.buttons[12].pressed) {
                player_buttons[player_index] |= 1 << 4;
            }
            if (gp.axes[1] > 0.5 || gp.buttons[13].pressed) {
                player_buttons[player_index] |= 1 << 5;
            }
            if (gp.axes[0] < -0.5 || gp.buttons[14].pressed) {
                player_buttons[player_index] |= 1 << 6;
            }
            if (gp.axes[0] > 0.5 || gp.buttons[15].pressed) {
                player_buttons[player_index] |= 1 << 7;
            }
        }
        if (gp1) set_buttons_down(gp1, 0);
        if (gp2) set_buttons_down(gp2, 1);

        if (!osd_enabled) {
            accum += delta;

            while (accum >= target_ms) {
                tick(nees_state_ptr, fb_ptr, player_buttons[0], player_buttons[1]);
                accum -= target_ms;
                need_render = true;
            }
        } else {
            need_render = true;
        }

        if (need_render) {
            draw(fb_u8);
            need_render = false;
        }

        requestAnimationFrame(render);
    })();
};