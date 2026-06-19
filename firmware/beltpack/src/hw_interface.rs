use crate::device::Device;
use core::fmt;
use protocol::{AudioPacket, DeviceConfig};

#[derive(Debug, Default, Clone, Eq, PartialEq, Copy)]
pub enum InputState {
    #[default]
    Nil = 0,
    A = 1,
    B = 2,
    C = 4,
    AB = 3,
    BC = 6,
    AC = 5,
    ABC = 7,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Copy)]
pub enum IndicatorState {
    #[default]
    Blank,
    Listening(bool, bool, bool),
    Talking(u8),
    LowBattery,
    NoConnection,
    Configuring,
}

impl fmt::Display for IndicatorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum HardwareError {
    NetworkBind,
    NetworkRecv,
    NetworkSend,
    NetworkParse,
    NetworkEncode,
}

pub trait HWImplementation {
    fn get_input_state(&mut self) -> InputState;
    fn set_indicator_state(&mut self, state: IndicatorState);
    fn read_mic_buffer(&mut self, out: &mut [i16]) -> usize;
    fn write_speaker_buffer(&mut self, buf: &[i16]) -> usize;
    fn init_hardware(&mut self);

    fn network_recv(&mut self, buf: &mut [u8]) -> Result<usize, HardwareError>;
    fn network_send(&mut self, buf: &[u8]) -> Result<(), HardwareError>;

    fn configure(&mut self) -> Option<DeviceConfig>;

    fn try_new() -> Result<Self, HardwareError>
    where
        Self: std::marker::Sized;
}
