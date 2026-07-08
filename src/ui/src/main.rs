mod device_state;

#[cfg(all(not(feature = "usbmon"), not(feature = "real")))]
compile_error!("must use either real device or usbmon");

#[cfg(all(feature = "usbmon", feature = "real"))]
compile_error!("can't use both real device and usbmon together");

#[cfg(feature = "usbmon")]
pub mod device_usbmon;

#[cfg(feature = "usbmon")]
use device_usbmon as device;
use std::collections::HashMap;

#[cfg(feature = "real")]
pub mod device;

use crate::device_state::Command::Set;
use crate::device_state::{Channel, ParameterValue, State, device_model};
use eframe::egui::{Slider, Ui};
use eframe::{Frame, egui};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
// use armas::prelude::*;
// use armas_audio::{MixerStrip, MixerStripMode};

fn main() -> eframe::Result {
    env_logger::init();

    let (usb_events, usb_updates) = device::init_capture().unwrap();

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
                device_state::Parameter::Meter(meter) => {
                    let value = device_state.meters.get(meter.id).copied().unwrap_or(0.0);
                    ui.add(egui::ProgressBar::new(value));
                    // ui.add()
                }
                device_state::Parameter::Button(button) => {
                    let value = device_state
                        .buttons
                        .get(button.id)
                        .copied()
                        .unwrap_or(false);
                    if ui
                        .add(egui::Button::new(button.label).fill(if value {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GRAY
                        }))
                        .clicked()
                    {
                        sender
                            .send(Set(ParameterValue::Button(button.clone(), !value)))
                            .unwrap();
                    }
                }
                device_state::Parameter::Knob(knob) => {
                    let mut value = device_state.knobs.get(knob.id).copied().unwrap_or(0.0);
                    if ui
                        // .add(Knob::pan(&mut value).range(knob.range.clone()))
                        .add(Slider::new(&mut value, knob.range.clone()))
                        .changed()
                    {
                        sender
                            .send(Set(ParameterValue::Knob(knob.clone(), value)))
                            .unwrap();
                    }
                }
            });
        });
    });
}

#[derive(Debug, Clone, Default)]
struct DeviceState {
    meters: HashMap<&'static str, f32>,
    buttons: HashMap<&'static str, bool>,
    knobs: HashMap<&'static str, f32>,
}

struct App {
    device_model: device_state::DeviceModel,
    usb_events: mpsc::Receiver<State>,
    usb_updates: mpsc::SyncSender<device_state::Command>,

    device_state: DeviceState,
}

fn drain_usb_events(rx: &std::sync::mpsc::Receiver<State>, state: &mut DeviceState) {
    while let Ok(event) = rx.try_recv() {
        match event {
            State::Update(param) => match param {
                device_state::ParameterValue::Meter(meter, value) => {
                    state.meters.insert(meter.id, value);
                }
                device_state::ParameterValue::Button(button, value) => {
                    state.buttons.insert(button.id, value);
                }
                device_state::ParameterValue::Knob(knob, value) => {
                    state.knobs.insert(knob.id, value);
                }
            },
            State::UpdateMany(params) => {
                for param in params {
                    match param {
                        device_state::ParameterValue::Meter(meter, value) => {
                            state.meters.insert(meter.id, value);
                        }
                        device_state::ParameterValue::Button(button, value) => {
                            state.buttons.insert(button.id, value);
                        }
                        device_state::ParameterValue::Knob(knob, value) => {
                            state.knobs.insert(knob.id, value);
                        }
                    }
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
