use crate::hw_interface::{HWImplementation, IndicatorState, InputState};
use protocol::OpusHandler;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};

pub struct LinuxDevice {
    shared_app_state: Arc<Mutex<SharedAppState>>,
    out_sink: Sink,
    out_stream: OutputStream,
    out_writer: SyncSender<f32>,
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
    fn new() -> Self {
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

        Self {
            shared_app_state,
            out_sink: sink,
            out_stream: stream,
            out_writer: out_write,
        }
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

    fn read_audio_buffer(&mut self, out: &mut [i16]) -> usize {
        0
    }

    fn write_audio_buffer(&mut self, buf: &[i16]) -> usize {
        for sample in buf {
            self.out_writer.send(*sample as f32 / 32767.0).unwrap();
        }
        buf.len()
    }

    fn init_hardware(&mut self) {}
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
        ui.label(state.indicator_state.to_string());

        if state.dirty {
            state.dirty = false;
            ui.ctx().request_repaint();
        }
    }
}
