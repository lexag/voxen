use crate::device::Device;
use core::fmt;

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
    Listening,
    Talking,
    LowBattery,
    NoConnection,
}

impl fmt::Display for IndicatorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

pub trait HWImplementation {
    fn get_input_state(&mut self) -> InputState;
    fn set_indicator_state(&mut self, state: IndicatorState);
    fn read_audio_buffer(&mut self, out: &mut [i16]) -> usize;
    fn write_audio_buffer(&mut self, buf: &[i16]) -> usize;
    fn init_hardware(&mut self);

    fn new() -> Self;
}
