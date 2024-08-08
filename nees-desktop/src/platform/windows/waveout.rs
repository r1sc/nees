use std::{borrow::BorrowMut, collections::VecDeque, mem::size_of};
use windows::{
    core::PSTR,
    Win32::Media::{
        self,
        Audio::{
            waveOutClose, waveOutPrepareHeader, waveOutReset, waveOutUnprepareHeader, waveOutWrite,
            HWAVEOUT, WAVEHDR,
        },
        MMSYSERR_NOERROR, MM_WOM_DONE,
    },
};

pub struct WaveoutDevice {
    shutting_down: bool,
    handle: HWAVEOUT,
    audio_headers: Vec<WAVEHDR>,
    queue: VecDeque<usize>,
    pub buffers: Vec<Vec<i16>>,
}

impl WaveoutDevice {
    pub fn new(num_buffers: usize, sample_rate: u32, buffer_size: usize) -> Box<Self> {
        let n_channels = 1;
        let w_bits_pers_sample = 16;
        let n_block_align = (n_channels * w_bits_pers_sample) / 8;
        let n_avg_bytes_per_sec = sample_rate * n_block_align;

        let format = Media::Audio::WAVEFORMATEX {
            wFormatTag: Media::Audio::WAVE_FORMAT_PCM as u16,
            nChannels: n_channels as u16,
            nSamplesPerSec: sample_rate,
            nAvgBytesPerSec: n_avg_bytes_per_sec,
            nBlockAlign: n_block_align as u16,
            wBitsPerSample: w_bits_pers_sample as u16,
            cbSize: 0,
        };

        let mut audio_headers = Vec::new();
        let mut buffers = Vec::new();
        let mut queue = VecDeque::new();

        // Prepare buffers
        for i in 0..num_buffers {
            let mut buffer: Vec<i16> = vec![0; buffer_size];

            let header = Media::Audio::WAVEHDR {
                lpData: PSTR(buffer.as_mut_ptr() as *mut u8),
                dwBufferLength: (buffer_size * size_of::<i16>()) as u32,
                dwFlags: 0,
                dwUser: i,
                ..Default::default()
            };

            audio_headers.push(header);
            buffers.push(buffer);
            queue.push_back(i);
        }

        let mut this = Box::new(Self {
            shutting_down: false,
            handle: HWAVEOUT::default(),
            queue,
            audio_headers,
            buffers,
        });

        unsafe {
            let result = Media::Audio::waveOutOpen(
                Some(&mut this.handle),
                Media::Audio::WAVE_MAPPER,
                &format,
                waveoutproc as *const fn() as usize,
                &*this as *const Self as usize,
                Media::Audio::CALLBACK_FUNCTION,
            );
            if result != Media::MMSYSERR_NOERROR {
                panic!("Failed to open waveout: Error: {}", result);
            }

            for audio_header in &mut this.audio_headers {
                let audio_header_ptr = audio_header as *mut WAVEHDR;

                let result = waveOutPrepareHeader(this.handle, audio_header_ptr, size_of::<WAVEHDR>() as u32);
                if result != MMSYSERR_NOERROR {
                    panic!("Error preparing header: {}", result);
                }
            }
        }

        this
    }

    fn on_waveout_proc(&mut self, u_msg: u32, dw_param1: usize, _dw_param2: usize) {
        if u_msg == MM_WOM_DONE && !self.shutting_down {
            let audio_header = dw_param1 as *mut WAVEHDR;

            let element_index = unsafe { (*audio_header).dwUser };
            self.queue.push_back(element_index);
        }
    }

    pub fn queue_buffer(&mut self) {
        let element_index = self.queue.pop_front().expect("Buffer queue is empty!?");

        let audio_header = &mut self.audio_headers[element_index];

        unsafe {
            let audio_header_ptr = audio_header as *mut WAVEHDR;

            let result =
                waveOutPrepareHeader(self.handle, audio_header_ptr, size_of::<WAVEHDR>() as u32);
            if result != MMSYSERR_NOERROR {
                panic!("Error preparing header: {}", result);
            }

            let result = waveOutWrite(self.handle, audio_header_ptr, size_of::<WAVEHDR>() as u32);
            if result != MMSYSERR_NOERROR {
                panic!("Error writing wave data: {}", result);
            }
        }
    }

    pub fn get_current_buffer(&mut self) -> Option<usize> {
        self.queue.front().copied()
    }
}

impl Drop for WaveoutDevice {
    fn drop(&mut self) {
        self.shutting_down = true;

        unsafe {
            waveOutReset(self.handle);
        }
        for audio_header in &mut self.audio_headers {
            unsafe {
                waveOutUnprepareHeader(
                    self.handle,
                    audio_header.borrow_mut() as *mut WAVEHDR,
                    size_of::<WAVEHDR>() as u32,
                );
            }
        }
        unsafe {
            waveOutClose(self.handle);
        }
    }
}

extern "stdcall" fn waveoutproc(
    _hwaveout: *const Media::Audio::HWAVEOUT,
    u_msg: u32,
    dw_instance: usize,
    dw_param1: usize,
    dw_param2: usize,
) {
    unsafe {
        let waveoutdevice = dw_instance as *mut WaveoutDevice;
        (*waveoutdevice).on_waveout_proc(u_msg, dw_param1, dw_param2);
    }
}
