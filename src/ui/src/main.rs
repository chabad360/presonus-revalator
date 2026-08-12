mod device_state;

#[cfg(all(not(feature = "usbmon"), not(feature = "real")))]
compile_error!("must use either real device or usbmon");

#[cfg(all(feature = "usbmon", feature = "real"))]
compile_error!("can't use both real device and usbmon together");

#[cfg(feature = "usbmon")]
pub mod device_usbmon;

#[cfg(feature = "usbmon")]
use device_usbmon as device;

#[cfg(feature = "real")]
pub mod device;

pub mod state;

use crate::device_state::Command::Set;
use crate::device_state::{BoolParameter, Channel, FloatParameter, Parameter, ParameterValue, State, device_model};
use eframe::egui::Slider;
use eframe::{Frame, egui};
use enum_map::EnumMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
// use armas::prelude::*;
// use armas_audio::{MixerStrip, MixerStripMode};

fn main() -> eframe::Result {
    env_logger::init();

    // let (usb_events, usb_updates) = device::init_capture();
    let init = device::init_capture();
    if let Err(err) = init {
        return eframe::run_simple_native(
            "Presonus Revalator",
            Default::default(),
            move |ctx, _frame| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label(format!("Error initializing device: {err:?}"));
                    if ui.button("Exit").clicked() {
                        std::process::exit(1);
                    }
                });
            });
    }
    let (usb_events, usb_updates) = init.unwrap();

    eframe::run_native(
        "Presonus Revalator",
        Default::default(),
        Box::new(|cc| {
            Ok(Box::new(App {
                device_model: device_model(),
                device_state: Default::default(),
                usb_events,
                usb_updates,
            }))
        }),
    )
}

fn channel(
    ui: &mut egui::Ui,
    device_state: &mut DeviceState,
    sender: mpsc::SyncSender<device_state::Command>,
    channel: Channel,
) {
    // let mut strip = MixerStrip::new(channel.name)
    //     .mode(MixerStripMode::Full)
    //     .width(80.0)
    //     .fader_level(0.75)
    //     .meter_level(device_state.meters.get("meter_1").copied().unwrap_or(0.0));
    //
    // strip.show(ui, &Theme::dark());

    ui.vertical(|ui| {
        ui.label(channel.name);
        ui.horizontal(|ui| {
            (channel).parameters.iter().for_each(|param| match param {
                Parameter::Knob(float_param, label) => {
                    let param_spec = float_param.spec();
                    if ui
                        .add(Slider::new(&mut device_state.floats[*float_param], param_spec.min..=param_spec.max))
                        .changed()
                    {
                        sender
                            .send(Set(ParameterValue::Float(*float_param, device_state.floats[*float_param])))
                            .unwrap();
                    }
                }
                Parameter::Meter(float_param, label) => {
                    let param_spec = float_param.spec();
                    ui.add(egui::ProgressBar::new(device_state.floats[*float_param]));
                }
                Parameter::Button(bool_param, label) => {
                    let param_spec = bool_param.spec();
                    let value = device_state.bools[*bool_param];
                    if ui
                        .add(egui::Button::new(*label).fill(if value {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GRAY
                        }))
                        .clicked()
                    {
                        sender
                            .send(Set(ParameterValue::Bool(*bool_param, !value)))
                            .unwrap();
                    }
                }
            });
        });
    });
}

#[derive(Debug, Clone, Default)]
struct DeviceState {
    floats: EnumMap<FloatParameter, f32>,
    bools: EnumMap<BoolParameter, bool>,
}

struct App {
    device_model: device_state::DeviceModel,
    usb_events: mpsc::Receiver<State>,
    usb_updates: mpsc::SyncSender<device_state::Command>,

    device_state: DeviceState,
}

fn update_param(state: &mut DeviceState, param: ParameterValue) {
    match param {
        ParameterValue::Float(float_param, value) => {
            state.floats[float_param] = value;
        }
        ParameterValue::Bool(bool_param, value) => {
            state.bools[bool_param] = value;
        }
    }
}

fn drain_usb_events(rx: &std::sync::mpsc::Receiver<State>, state: &mut DeviceState) {
    while let Ok(event) = rx.try_recv() {
        match event {
            State::Update(param) => update_param(state, param),
            State::UpdateMany(params) => {
                for param in params {
                    update_param(state, param);
                }
            }
            State::None => {}
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // self.usb_updates
        //     .send(device_state::Command::Refresh())
        //     .unwrap();
        drain_usb_events(&self.usb_events, &mut self.device_state);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.device_model.channels.iter().for_each(|ch| {
                channel(
                    ui,
                    &mut self.device_state,
                    self.usb_updates.clone(),
                    ch.clone(),
                );
            });
        });

        thread::sleep(Duration::from_millis(14));
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
