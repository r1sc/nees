const NUM_BUFFERS = 4;
const BUFFER_LEN = 128;

class NesAudioProcessor extends AudioWorkletProcessor {
  to_play: number[] = [];
  queue: number[] = [];
  buffers: Int16Array[] = [];

  constructor() {
    super();

    for (let i = 0; i < NUM_BUFFERS; i++) {
      this.queue.push(i);
    }

    this.port.onmessage = (event: MessageEvent<{ samples: Int16Array }>) => {
      if (this.queue.length === 0) return;

      const current_buffer_index = this.queue[0];
      this.buffers[current_buffer_index] = event.data.samples;
      this.queue_buffer();
    }
  }

  queue_buffer() {
    const element_index = this.queue.shift();
    if (element_index === undefined) {
      throw new Error("Buffer queue is empty!?");
    }
    this.to_play.push(element_index);
  }

  process(inputs: Float32Array[][], outputs: Float32Array[][], parameters: Record<string, Float32Array>): boolean {
    const channel = outputs[0][0];

    const buffer_index = this.to_play.shift();
    if (buffer_index === undefined) {
      channel.fill(0);
      return true;
    }

    for (let i = 0; i < channel.length; i++) {
      channel[i] = this.buffers[buffer_index][i] / 32768;
    }

    this.queue.push(buffer_index);

    return true;
  }
}

registerProcessor("nes-audio-processor", NesAudioProcessor);
