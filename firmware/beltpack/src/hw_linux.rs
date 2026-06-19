use crate::hw_interface::{HWImplementation, HardwareError, IndicatorState, InputState};
use egui::Color32;
use protocol::{AudioPacket, OpusHandler};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use std::{
    net::{Ipv4Addr, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
    },
    time::Duration,
};

pub struct LinuxDevice {
    shared_app_state: Arc<Mutex<SharedAppState>>,
    out_sink: Sink,
    out_stream: OutputStream,
    out_writer: SyncSender<f32>,
    net_socket: UdpSocket,
}

struct ChannelSource {
    rx: Receiver<f32>,
    channels: u16,
    sample_rate: u32,
}

impl Iterator for ChannelSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.rx.recv().ok()
    }
}

impl rodio::Source for ChannelSource {
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl HWImplementation for LinuxDevice {
    fn try_new() -> Result<Self, HardwareError> {
        // Setup GUI
        let shared_app_state = Arc::new(Mutex::new(SharedAppState::default()));
        let state = shared_app_state.clone();
        std::thread::spawn(move || {
            //use winit::platform::wayland::EventLoopBuilderExtWayland;
            use winit::platform::x11::EventLoopBuilderExtX11;

            let native_options = eframe::NativeOptions {
                event_loop_builder: Some(Box::new(|builder| {
                    builder.with_any_thread(true);
                })),
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([400.0, 300.0])
                    .with_min_inner_size([300.0, 220.0]),
                ..Default::default()
            };
            let _ = eframe::run_native(
                "eframe template",
                native_options,
                Box::new(|cc| Ok(Box::new(App::new(cc, state)))),
            );
        });

        // Setup audio I/O
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let (out_write, out_read) = mpsc::sync_channel::<f32>(OpusHandler::AUDIO_BUFFER_SIZE);

        let source = ChannelSource {
            rx: out_read,
            channels: 1,
            sample_rate: OpusHandler::SAMPLE_RATE,
        };

        sink.append(source);
        let net_socket =
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|_| HardwareError::NetworkBind)?;
        let _ = net_socket.set_nonblocking(true);

        Ok(Self {
            net_socket,
            shared_app_state,
            out_sink: sink,
            out_stream: stream,
            out_writer: out_write,
        })
    }

    fn get_input_state(&mut self) -> crate::hw_interface::InputState {
        let state = self.shared_app_state.lock().unwrap();
        state.input_state
    }

    fn set_indicator_state(&mut self, state: crate::hw_interface::IndicatorState) {
        let mut shared_state = self.shared_app_state.lock().unwrap();
        shared_state.indicator_state = state;
        shared_state.dirty = true;
    }

    fn read_mic_buffer(&mut self, out: &mut [i16]) -> usize {
        0
    }

    fn write_speaker_buffer(&mut self, buf: &[i16]) -> usize {
        for sample in buf {
            self.out_writer.send(*sample as f32 / 32767.0).unwrap();
        }
        buf.len()
    }

    fn init_hardware(&mut self) {}

    fn network_recv(&mut self, buf: &mut [u8]) -> Result<usize, HardwareError> {
        let res = self.net_socket.recv_from(buf);
        match res {
            Ok((size, _)) => return Ok(size),
            // FIXME: this implies that all errors are to be understood as "no audio packet came in
            // this time" which is mostly true, but we want to handle a few errors, like losing
            // network connection, so we can indicate them to the user
            Err(_) => return Ok(0),
        };
    }

    fn network_send(&mut self, buf: &[u8]) -> Result<(), HardwareError> {
        self.net_socket
            .send(buf)
            .map_err(|_| HardwareError::NetworkSend)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SharedAppState {
    dirty: bool,
    input_state: InputState,
    indicator_state: IndicatorState,
}

#[derive(Debug, Default)]
pub struct App {
    state: Arc<Mutex<SharedAppState>>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        shared_app_state: Arc<Mutex<SharedAppState>>,
    ) -> Self {
        Self {
            state: shared_app_state,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();

        state.input_state = InputState::Nil;
        if ui.button("A").is_pointer_button_down_on() {
            state.input_state = InputState::A
        }
        if ui.button("B").is_pointer_button_down_on() {
            state.input_state = InputState::B
        }
        if ui.button("C").is_pointer_button_down_on() {
            state.input_state = InputState::C
        }
        if ui.button("AB").is_pointer_button_down_on() {
            state.input_state = InputState::AB
        }
        if ui.button("BC").is_pointer_button_down_on() {
            state.input_state = InputState::BC
        }
        if ui.button("AC").is_pointer_button_down_on() {
            state.input_state = InputState::AC
        }
        if ui.button("ABC").is_pointer_button_down_on() {
            state.input_state = InputState::ABC
        }
        ui.horizontal(|ui| match state.indicator_state {
            IndicatorState::Blank => {
                ui.colored_label(Color32::GRAY, "A");
                ui.colored_label(Color32::GRAY, "B");
                ui.colored_label(Color32::GRAY, "C");
            }
            IndicatorState::Listening(a, b, c) => {
                ui.colored_label(if a { Color32::CYAN } else { Color32::GRAY }, "A");
                ui.colored_label(if b { Color32::CYAN } else { Color32::GRAY }, "B");
                ui.colored_label(if c { Color32::CYAN } else { Color32::GRAY }, "C");
            }
            IndicatorState::Talking(target) => {
                ui.colored_label(
                    if target == 1 {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::GRAY
                    },
                    "A",
                );
                ui.colored_label(
                    if target == 2 {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::GRAY
                    },
                    "B",
                );
                ui.colored_label(
                    if target == 3 {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::GRAY
                    },
                    "C",
                );
            }
            IndicatorState::LowBattery => {
                ui.colored_label(Color32::RED, "A");
                ui.colored_label(Color32::RED, "B");
                ui.colored_label(Color32::RED, "C");
            }
            IndicatorState::NoConnection => {
                ui.colored_label(Color32::MAGENTA, "A");
                ui.colored_label(Color32::MAGENTA, "B");
                ui.colored_label(Color32::MAGENTA, "C");
            }
        });

        if state.dirty {
            state.dirty = false;
        }
        ui.ctx().request_repaint();
    }
}
