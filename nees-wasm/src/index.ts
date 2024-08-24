import { make_renderer } from "./renderer";
import wasminit, { draw_osd, get_framebuffer_ptr, init, load_state, save_state, step_osd, StepResponse, tick } from "../pkg/nees_wasm";
import wasm_path from "../pkg/nees_wasm_bg.wasm";

// Get rom path from query string
const rom_path = new URLSearchParams(window.location.search).get("rom");

declare global {
    interface Window {
        waveout_callback: (sample: number) => void;
    }
}

async function get_rom_from_url(url: string) {
    const response = await fetch(url);
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
}

if (rom_path) {
    const start_btn = document.createElement("button");
    start_btn.textContent = "Click to start";
    document.body.appendChild(start_btn);
    start_btn.addEventListener("click", async () => {
        start_btn.remove();

        const rom = await get_rom_from_url(rom_path);

        start(rom, rom_path);
    });
} else {
    const file_input = document.createElement("input");
    file_input.type = "file";
    file_input.accept = ".nes";
    file_input.textContent = "Select a .nes file";

    document.body.appendChild(file_input);
    file_input.addEventListener("change", async () => {
        file_input.remove();

        if (file_input.files && file_input.files.length > 0) {
            const rom = new Uint8Array(await file_input.files[0].arrayBuffer());
            start(rom, file_input.files[0].name);
        }
    });
}

async function start(rom: Uint8Array, rom_name: string) {
    const audio = new AudioContext({ sampleRate: 15720 });
    await audio.audioWorklet.addModule("nes-audio-processor.js");
    const audioNode = new AudioWorkletNode(audio, "nes-audio-processor", {
        channelCount: 1,
        channelCountMode: "explicit",
    });
    audioNode.connect(audio.destination);

    const current_samples = new Int16Array(128);
    let current_sample_index = 0;
    window.waveout_callback = (sample: number) => {
        current_samples[current_sample_index++] = sample;
        if (current_sample_index === 128) {
            audioNode.port.postMessage({ samples: current_samples });
            current_sample_index = 0;
        }
    };

    async function loadwasm() {
        const response = fetch(wasm_path as unknown as string);
        return await wasminit(response);
    }

    const o = await loadwasm();

    const nees_state_ptr = init(rom);
    const fb_ptr = get_framebuffer_ptr();
    const fb_u8 = new Uint8Array(o.memory.buffer, fb_ptr, 256 * 240 * 4);

    const renderer = make_renderer();

    let last_time = 0;
    let accum = 0;
    const target_ms = 1000 / 60;

    let player_buttons: [number, number] = [0, 0];
    const serialized_keys = JSON.parse(localStorage.getItem("keys") ?? "null");
    const keys = serialized_keys ?? {
        b_button: ["k", "a"],
        a_button: ["l", "s"],
        select_button: ["i", "q"],
        start_button: ["o", "w"],
        up_button: ["ArrowUp", "t"],
        down_button: ["ArrowDown", "g"],
        left_button: ["ArrowLeft", "f"],
        right_button: ["ArrowRight", "h"],
    };

    let osd_enabled = false;

    const save = () => {
        const save_data = save_state(nees_state_ptr);
        localStorage.setItem(rom_name, save_data.reduce((acc, val) => acc + String.fromCharCode(val), ""));
    };

    const load = () => {
        const save_data = localStorage.getItem(rom_name);
        if (save_data) {
            const save_data_u8 = new Uint8Array(save_data.length);
            for (let i = 0; i < save_data.length; i++) {
                save_data_u8[i] = save_data.charCodeAt(i);
            }
            load_state(nees_state_ptr, save_data_u8);
        }
    };

    window.addEventListener("keydown", (e) => {
        for (let i = 0; i < 2; i++) {
            if (e.key === keys.a_button[i]) {
                player_buttons[i] |= 1 << 0;
            } else if (e.key === keys.b_button[i]) {
                player_buttons[i] |= 1 << 1;
            } else if (e.key === keys.select_button[i]) {
                player_buttons[i] |= 1 << 2;
            } else if (e.key === keys.start_button[i]) {
                player_buttons[i] |= 1 << 3;
            } else if (e.key === keys.up_button[i]) {
                player_buttons[i] |= 1 << 4;
            } else if (e.key === keys.down_button[i]) {
                player_buttons[i] |= 1 << 5;
            } else if (e.key === keys.left_button[i]) {
                player_buttons[i] |= 1 << 6;
            } else if (e.key === keys.right_button[i]) {
                player_buttons[i] |= 1 << 7;
            }
        }

        if (e.key === "F2") {
            e.preventDefault();
            save();
        } else if (e.key === "F3") {
            e.preventDefault();
            load();
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

            if (response.action >= 1 && response.action <= 8) {
                if (response.action === 1) keys.b_button[response.value] = e.key;
                else if (response.action === 2) keys.a_button[response.value] = e.key;
                else if (response.action === 3) keys.select_button[response.value] = e.key;
                else if (response.action === 4) keys.start_button[response.value] = e.key;
                else if (response.action === 5) keys.up_button[response.value] = e.key;
                else if (response.action === 6) keys.down_button[response.value] = e.key;
                else if (response.action === 7) keys.left_button[response.value] = e.key;
                else if (response.action === 8) keys.right_button[response.value] = e.key;

                localStorage.setItem("keys", JSON.stringify(keys));
            }
            else if (response.action === 9) { save(); osd_enabled = false; }
            else if (response.action === 10) { load(); osd_enabled = false; }
            else if (response.action === 11) {
                renderer.set_horizontal_adjustment(response.value / 256);
            }

            draw_osd(nees_state_ptr, fb_ptr);
        }
    });

    window.addEventListener("keyup", (e) => {
        for (let i = 0; i < 2; i++) {
            if (e.key === keys.a_button[i]) {
                player_buttons[i] &= ~(1 << 0);
            } else if (e.key === keys.b_button[i]) {
                player_buttons[i] &= ~(1 << 1);
            } else if (e.key === keys.select_button[i]) {
                player_buttons[i] &= ~(1 << 2);
            } else if (e.key === keys.start_button[i]) {
                player_buttons[i] &= ~(1 << 3);
            } else if (e.key === keys.up_button[i]) {
                player_buttons[i] &= ~(1 << 4);
            } else if (e.key === keys.down_button[i]) {
                player_buttons[i] &= ~(1 << 5);
            } else if (e.key === keys.left_button[i]) {
                player_buttons[i] &= ~(1 << 6);
            } else if (e.key === keys.right_button[i]) {
                player_buttons[i] &= ~(1 << 7);
            }
        }
    });

    (function render(now: DOMHighResTimeStamp) {

        let delta = now - last_time;
        last_time = now;

        if (delta > 500) {
            delta = 500;
        }


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


        if (!osd_enabled) {
            accum += delta;

            while (accum >= target_ms) {
                const [gp1, gp2] = navigator.getGamepads();
                if (gp1) set_buttons_down(gp1, 0);
                if (gp2) set_buttons_down(gp2, 1);

                tick(nees_state_ptr, fb_ptr, player_buttons[0], player_buttons[1]);
                accum -= target_ms;
            }
        }

        renderer.draw(fb_u8);

        requestAnimationFrame(render);
    })(0);
};