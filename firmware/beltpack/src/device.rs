use crate::hw_interface::{HWImplementation, HardwareError, IndicatorState, InputState};
#[cfg(feature = "desktop")]
use crate::hw_linux::LinuxDevice;
use core::f32;
use protocol::{AudioPacket, DeviceConfig, OpusHandler, PacketAddress};
use std::net::{Ipv4Addr, SocketAddrV4};

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
    pub hardware: HWDevice,
    pub indication: IndicatorState,
    pub target: u8,
    pub listen_src: (bool, bool, bool),
    pub talk_buffer: ConstRingBuffer<i16, { OpusHandler::AUDIO_BUFFER_SIZE * 3 }>,
    pub listen_buffer: ConstRingBuffer<i16, { OpusHandler::AUDIO_BUFFER_SIZE * 3 }>,
    pub opus_handler: OpusHandler,
    pub current_input: (InputState, usize),
    pub phase: f32,
    pub config: DeviceConfig,
    pub out_sequence_number: u16,
}

impl Device {
    /// How many samples are sent to hardware audio each loop. Lower => less latency, but needs
    /// faster cycle frequency to keep up
    const HW_AUDIO_CHUNK_SIZE: usize = 32;

    pub fn try_new() -> Result<Self, HardwareError> {
        Ok(Self {
            target: 0,
            listen_src: (false, false, false),
            config: DeviceConfig::default(),
            hardware: HWDevice::try_new()?,
            indication: IndicatorState::Blank,
            talk_buffer: ConstRingBuffer::new(0),
            listen_buffer: ConstRingBuffer::new(0),
            opus_handler: OpusHandler::new(),
            phase: 0.0,
            current_input: (InputState::Nil, 0),
            out_sequence_number: 0,
        })
    }

    pub fn start(&mut self) {
        self.hardware.init_hardware();
        self.indicate(IndicatorState::NoConnection);
        loop {
            self.tick();
        }
    }

    pub fn tick(&mut self) {
        const BUTTON_HOLD_THRESHOLD: usize = 300;
        const BUTTON_LONGHOLD_THRESHOLD: usize = 3000;
        let inp = self.input();
        if inp == self.current_input.0 {
            // Holding
            self.current_input.1 += 1;
            if inp != InputState::Nil {
                if inp == InputState::AC && self.current_input.1 > BUTTON_LONGHOLD_THRESHOLD {
                    self.indicate(IndicatorState::Configuring);
                    self.enter_configuration_mode();
                }
            }
        } else if self.current_input.0 == InputState::Nil {
            // Press
            self.current_input.1 = 0;
            match inp {
                InputState::A => self.target = 1,
                InputState::B => self.target = 2,
                InputState::C => self.target = 3,
                _ => {}
            }
        } else if inp == InputState::Nil {
            // Released
            if self.current_input.1 > BUTTON_HOLD_THRESHOLD {
                // After hold
                self.target = 0;
            } else {
                // After tap
            }
        }

        self.current_input.0 = inp;

        if self.target != 0 {
            self.indicate(IndicatorState::Talking(self.target))
        } else {
            self.indicate(IndicatorState::Listening(
                self.listen_src.0,
                self.listen_src.1,
                self.listen_src.2,
            ))
        }

        // read microphone into talk buffer
        self.listen_from_microphone();

        // write audio into speaker buffer, but only if there is stuff to write.
        if self.listen_buffer.len() >= Self::HW_AUDIO_CHUNK_SIZE {
            self.write_to_speaker();
        }

        // refill the speaker buffer
        while self.listen_buffer.free_capacity() >= OpusHandler::AUDIO_BUFFER_SIZE {
            if self.listen_from_network().is_none() {
                break;
            };
        }

        // send mic buffer to base if speaking
        if self.target != 0 && self.talk_buffer.len() > OpusHandler::AUDIO_BUFFER_SIZE {
            self.write_to_network();
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

    fn write_to_speaker(&mut self) -> Option<()> {
        let mut speaker_audio_buf = [0; 4096];
        self.listen_buffer
            .pop_slice(&mut speaker_audio_buf[..Self::HW_AUDIO_CHUNK_SIZE]);
        self.hardware
            .write_speaker_buffer(&speaker_audio_buf[..Self::HW_AUDIO_CHUNK_SIZE]);
        Some(())
    }

    fn listen_from_microphone(&mut self) -> Option<()> {
        let mut mic_audio_buf = [0; 4096];
        let buf_size = self.hardware.read_mic_buffer(&mut mic_audio_buf);
        self.talk_buffer.push_slice(&mic_audio_buf[..buf_size]);
        Some(())
    }

    fn listen_from_network(&mut self) -> Option<()> {
        const PACKET_SIZE: usize = size_of::<AudioPacket>();
        let mut packet_buf = [0; PACKET_SIZE];
        let size = self.hardware.network_recv(&mut packet_buf).ok()?;
        // FIXME: no verification is done that it's actually the base sending the packet. Might be
        // worth looking into.

        if size != PACKET_SIZE {
            return None;
        }

        let res = postcard::from_bytes::<AudioPacket>(&packet_buf);
        if let Ok(packet) = res {
            let mut network_listen_buffer = [0; OpusHandler::AUDIO_BUFFER_SIZE];
            self.opus_handler.decode(
                &packet.opus_packet,
                packet.opus_size as usize,
                &mut network_listen_buffer,
            );
            self.listen_buffer.push_slice(&network_listen_buffer);
            return Some(());
        }
        None
    }

    fn write_to_network(&mut self) -> Option<()> {
        let mut network_talk_opus_buffer = [0; OpusHandler::OPUS_BUFFER_SIZE];
        let mut network_talk_buffer = [0; OpusHandler::AUDIO_BUFFER_SIZE];

        self.talk_buffer.pop_slice(&mut network_talk_buffer);
        let size = self
            .opus_handler
            .encode(&network_talk_buffer, &mut network_talk_opus_buffer)?;

        let packet = AudioPacket {
            address: PacketAddress::new(0, self.target.into())?,
            opus_packet: network_talk_opus_buffer,
            opus_size: size as u16,
            sequence: self.out_sequence_number,
            timestamp: 0,
        };
        const PACKET_SIZE: usize = size_of::<AudioPacket>();
        let mut packet_buf = [0_u8; PACKET_SIZE];

        postcard::to_slice(&packet, &mut packet_buf);

        self.hardware.network_send(&packet_buf);
        Some(())
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

    fn enter_configuration_mode(&mut self) {
        if let Some(config) = self.hardware.configure() {
            self.config = config;
        }
    }
}
