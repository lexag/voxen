use crate::hw_interface::{HWImplementation, IndicatorState, InputState};
#[cfg(feature = "desktop")]
use crate::hw_linux::LinuxDevice;
use core::f32;
use protocol::OpusHandler;

#[cfg(feature = "desktop")]
type HWDevice = LinuxDevice;
#[cfg(feature = "embedded")]
type HWDevice = STMDevice;

#[derive(Debug)]
pub struct ConstRingBuffer<T, const N: usize> {
    buffer: [T; N],
    next_write_idx: usize,
    next_read_idx: usize,
}

impl<T: Copy, const N: usize> ConstRingBuffer<T, N> {
    pub fn new(fill: T) -> Self {
        Self {
            buffer: [fill; N],
            next_read_idx: N / 2,
            next_write_idx: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buffer[self.next_write_idx] = item;
        self.next_write_idx += 1;
        self.next_write_idx %= N;
    }

    pub fn pop(&mut self) -> T {
        let item = self.buffer[self.next_read_idx];
        //if self.next_read_idx == self.next_write_idx {
        //    self.next_write_idx += 1;
        //    self.next_write_idx %= N;
        //}
        self.next_read_idx += 1;
        self.next_read_idx %= N;
        item
    }

    pub fn push_slice(&mut self, slice: &[T]) {
        for element in slice {
            self.push(*element);
        }
    }

    pub fn pop_slice(&mut self, slice: &mut [T]) {
        for element in slice {
            *element = self.pop()
        }
    }

    pub fn len(&self) -> usize {
        if self.next_write_idx >= self.next_read_idx {
            self.next_write_idx - self.next_read_idx
        } else {
            self.next_write_idx + N - self.next_read_idx
        }
    }

    pub fn free_capacity(&self) -> usize {
        N - self.len()
    }
}

pub struct Device {
    pub device_number: u8,
    pub hardware: HWDevice,
    pub indication: IndicatorState,
    pub target: u8,
    pub talk_buffer: ConstRingBuffer<i16, { OpusHandler::AUDIO_BUFFER_SIZE * 3 }>,
    pub listen_buffer: ConstRingBuffer<i16, { OpusHandler::AUDIO_BUFFER_SIZE * 3 }>,
    pub opus_handler: OpusHandler,
    pub phase: f32,
}

impl Device {
    /// How many samples are sent to hardware audio each loop. Lower => less latency, but needs
    /// faster cycle frequency to keep up
    const HW_AUDIO_CHUNK_SIZE: usize = 32;

    pub fn new() -> Self {
        Self {
            target: 0,
            device_number: 0,
            hardware: HWDevice::new(),
            indication: IndicatorState::Blank,
            talk_buffer: ConstRingBuffer::new(0),
            listen_buffer: ConstRingBuffer::new(0),
            opus_handler: OpusHandler::new(),
            phase: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.hardware.init_hardware();
        self.indicate(IndicatorState::NoConnection);
        loop {
            self.tick();
        }
    }

    pub fn tick(&mut self) {
        let inp = self.input();
        match inp {
            InputState::A => self.target = 1,
            InputState::B => self.target = 2,
            InputState::C => self.target = 3,
            _ => self.target = 0,
        }

        if self.target != 0 {
            self.indicate(IndicatorState::Talking)
        } else {
            self.indicate(IndicatorState::Listening)
        }

        let mut mic_audio_buf = [0; 4096];
        let buf_size = self.hardware.read_audio_buffer(&mut mic_audio_buf);
        self.talk_buffer.push_slice(&mic_audio_buf[..buf_size]);

        let mut speaker_audio_buf = [0; 4096];
        if self.listen_buffer.len() >= Self::HW_AUDIO_CHUNK_SIZE {
            self.listen_buffer
                .pop_slice(&mut speaker_audio_buf[..Self::HW_AUDIO_CHUNK_SIZE]);
            self.hardware
                .write_audio_buffer(&speaker_audio_buf[..Self::HW_AUDIO_CHUNK_SIZE]);
        }

        if self.listen_buffer.free_capacity() >= OpusHandler::AUDIO_BUFFER_SIZE {
            if self.target != 0 {
                for t in (0..OpusHandler::AUDIO_BUFFER_SIZE)
                    .map(|x| x as f32 / OpusHandler::SAMPLE_RATE as f32)
                {
                    let freq = if self.target == 0 {
                        440.0
                    } else {
                        880.0 * self.target as f32 / 3.0
                    };
                    let sample = (self.phase * 2.0 * f32::consts::PI).sin();
                    self.phase += freq / OpusHandler::SAMPLE_RATE as f32;
                    self.phase %= 1.0;
                    let amplitude = i16::MAX as f32 * 0.05;
                    self.listen_buffer.push((sample * amplitude) as i16);
                }
            } else {
                for _ in 0..OpusHandler::AUDIO_BUFFER_SIZE {
                    self.listen_buffer.push(0);
                }
            }
        }
        if self.listen_buffer.len() == 0 {
            println!("listen buffer ran out");
        }
    }

    fn input(&mut self) -> InputState {
        self.hardware.get_input_state()
    }

    fn indicate(&mut self, state: IndicatorState) {
        if self.indication != state {
            self.hardware.set_indicator_state(state);
            self.indication = state;
        }
    }
}
