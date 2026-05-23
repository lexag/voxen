pub struct AudioPacket {
    pub timestamp: u32,
    pub sequence: u16,
    pub address: PacketAddress,
    pub opus_packet: [u8; OpusHandler::OPUS_BUFFER_SIZE],
}

impl AudioPacket {}

pub struct OpusHandler {
    pub encoder: opus::Encoder,
    pub decoder: opus::Decoder,
}

impl Default for OpusHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl OpusHandler {
    const PACKET_INTERVAL_MS: u32 = 10;
    pub const SAMPLE_RATE: u32 = 48000;
    pub const AUDIO_BUFFER_SIZE: usize =
        (Self::PACKET_INTERVAL_MS * Self::SAMPLE_RATE / 1000) as usize;

    const OPUS_BITRATE: usize = 32000;
    pub const OPUS_BUFFER_SIZE: usize = 1276; // maximum buffer size according to someone?
                                              // i thought constant bitrate would be constant size but I guess not...
                                              // Self::OPUS_BITRATE / 8 * Self::PACKET_INTERVAL_MS as usize / 1000;

    const USE_FEC: bool = false;

    pub fn new() -> Self {
        let mut encoder = opus::Encoder::new(
            Self::SAMPLE_RATE,
            opus::Channels::Mono,
            opus::Application::Voip,
        )
        .unwrap();

        let _ = encoder.set_vbr(false);
        let _ = encoder.set_dtx(false);
        let _ = encoder.set_lsb_depth(16);
        let _ = encoder.set_bitrate(opus::Bitrate::Bits(Self::OPUS_BITRATE as i32));
        let _ = encoder.set_inband_fec(Self::USE_FEC);

        let decoder = opus::Decoder::new(Self::SAMPLE_RATE, opus::Channels::Mono).unwrap();

        println!(
            "Initializing OpusHandler with:
            {} samples audio buffer size
            {} samples max opus buffer size
            {} Hz sample rate",
            Self::AUDIO_BUFFER_SIZE,
            Self::OPUS_BUFFER_SIZE,
            Self::SAMPLE_RATE
        );

        Self { encoder, decoder }
    }

    pub fn make_audio_buffer() -> [i16; Self::AUDIO_BUFFER_SIZE] {
        [0_i16; Self::AUDIO_BUFFER_SIZE]
    }

    pub fn make_opus_buffer() -> [u8; Self::OPUS_BUFFER_SIZE] {
        [0_u8; Self::OPUS_BUFFER_SIZE]
    }

    pub fn encode(
        &mut self,
        buffer: &[i16; Self::AUDIO_BUFFER_SIZE],
        out: &mut [u8; Self::OPUS_BUFFER_SIZE],
    ) -> Option<usize> {
        let num_bytes = self.encoder.encode(buffer, out).unwrap();
        Some(num_bytes)
    }

    pub fn decode(
        &mut self,
        buffer: &[u8; Self::OPUS_BUFFER_SIZE],
        size: usize,
        out: &mut [i16; Self::AUDIO_BUFFER_SIZE],
    ) -> Option<usize> {
        let num_bytes = self
            .decoder
            .decode(&buffer[..size], out, Self::USE_FEC)
            .unwrap();
        assert_eq!(num_bytes, Self::AUDIO_BUFFER_SIZE);
        Some(num_bytes)
    }
}

use packed_struct::prelude::*;

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "msb0")]
pub struct PacketAddress {
    #[packed_field(bits = "0..=1", ty = "enum")]
    channel: DeviceChannel,
    #[packed_field(bits = "2..=7")]
    device_number: Integer<u8, packed_bits::Bits<6>>,
}

#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum DeviceChannel {
    Return = 0,
    A = 1,
    B = 2,
    C = 3,
}

impl PacketAddress {
    pub fn new(device: u8, channel: DeviceChannel) -> Option<Self> {
        if device > 0b0011_1111 {
            return None;
        };

        Some(Self {
            device_number: device.into(),
            channel,
        })
    }

    pub fn device(&self) -> u8 {
        self.device_number.into()
    }

    pub fn channel(&self) -> DeviceChannel {
        self.channel
    }
}
