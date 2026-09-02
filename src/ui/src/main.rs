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
use crate::device_state::{
    BoolParameters, ChoiceParameters, FloatParameters, Parameter, ParameterGroup, ParameterValue,
    State, UIntParameters, device_model,
};
use eframe::{Frame, egui};
use egui_audio_controls::{Fader, Knob, LevelMeter};
use enum_map::EnumMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() -> eframe::Result {
    env_logger::init();

    // let (usb_events, usb_updates) = device::init_capture();
    let init = device::init_capture();
    if let Err(err) = init {
        return eframe::run_ui_native(
            "Presonus Revalator",
            Default::default(),
            move |ui, _frame| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label(format!("Error initializing device: {err:?}"));
                    if ui.button("Exit").clicked() {
                        std::process::exit(1);
                    }
                });
            },
        );
    }
    let (usb_events, usb_updates) = init.unwrap();

    eframe::run_native(
        "Presonus Revalator",
        Default::default(),
        Box::new(|_cc| {
            Ok(Box::new(App {
                device_model: device_model(),
                device_state: Default::default(),
                usb_events,
                usb_updates,
            }))
        }),
    )
}

fn parameter_group(
    ui: &mut egui::Ui,
    device_state: &mut DeviceState,
    sender: &mpsc::SyncSender<device_state::Command>,
    group: &ParameterGroup,
) {
    ui.group(|ui| {
        ui.label(egui::RichText::new(group.name).strong());
        ui.horizontal_wrapped(|ui| {
            for parameter in &group.parameters {
                match parameter {
                    Parameter::Knob(float_parameter, label) => {
                        let spec = float_parameter.spec();
                        let mut knob = Knob::new(
                            &mut device_state.floats[*float_parameter],
                            spec.min..=spec.max,
                        )
                            .text(*label);
                        knob = match float_parameter {
                            FloatParameters::Gain(_) | FloatParameters::ReverbSend(_) => {
                                knob.suffix(" dB")
                            }
                            FloatParameters::MainGain() | FloatParameters::HeadphonesGain() => knob
                                .suffix("%")
                                .decimals(0)
                                .custom_formatter(|value, _| format!("{:.0}", value * 100.0))
                                .custom_parser(|text| {
                                    text.trim()
                                        .trim_end_matches('%')
                                        .parse::<f64>()
                                        .ok()
                                        .map(|value| value / 100.0)
                                }),
                            FloatParameters::Pan(_) => knob.notch(0.5),
                            FloatParameters::MonitorMix() => knob.notch(0.0),
                            _ => knob,
                        };
                        if ui.add(knob).changed() {
                            sender
                                .send(Set(ParameterValue::Float(
                                    *float_parameter,
                                    device_state.floats[*float_parameter],
                                )))
                                .unwrap();
                        }
                    }
                    Parameter::Fader(float_parameter, label) => {
                        let spec = float_parameter.spec();
                        let fader = Fader::new(
                            &mut device_state.floats[*float_parameter],
                            spec.min..=spec.max,
                        )
                            .text(*label)
                            .decibels()
                            .notch(0.0)
                            .reset_value(0.0);
                        if ui.add(fader).changed() {
                            sender
                                .send(Set(ParameterValue::Float(
                                    *float_parameter,
                                    device_state.floats[*float_parameter],
                                )))
                                .unwrap();
                        }
                    }
                    Parameter::Meter(channels, label) => {
                        let values = channels
                            .iter()
                            .map(|(parameter, _)| device_state.floats[*parameter])
                            .collect::<Vec<_>>();
                        let clipping = channels
                            .iter()
                            .map(|(_, clipping)| {
                                clipping.map(|parameter| device_state.bools[parameter])
                            })
                            .collect::<Vec<_>>();
                        ui.add(
                            LevelMeter::new(values)
                                .optional_channel_clipping(clipping)
                                .text(*label),
                        );
                    }
                    Parameter::InvertedMeter(channels, label) => {
                        let values = channels
                            .iter()
                            .map(|parameter| device_state.floats[*parameter])
                            .collect::<Vec<_>>();
                        ui.add(
                            LevelMeter::new(values)
                                .range_db(-60.0..=0.0)
                                .inverted()
                                .text(*label),
                        );
                    }
                    Parameter::UIntValue(uint_parameter, label) => {
                        let value = device_state.uints[*uint_parameter];
                        ui.label(format!("{label}: {value:#010x}"));
                    }
                    Parameter::Indicator(bool_parameter, label) => {
                        let value = device_state.bools[*bool_parameter];
                        ui.label(format!("{label}: {}", if value { "ON" } else { "OFF" }));
                    }
                    Parameter::Button(bool_parameter, label) => {
                        let value = device_state.bools[*bool_parameter];
                        if ui
                            .add(egui::Button::new(*label).fill(if value {
                                egui::Color32::GREEN
                            } else {
                                egui::Color32::GRAY
                            }))
                            .clicked()
                        {
                            sender
                                .send(Set(ParameterValue::Bool(*bool_parameter, !value)))
                                .unwrap();
                        }
                    }
                    Parameter::Choice(choice_parameter, label, choices) => {
                        let value = &mut device_state.choices[*choice_parameter];
                        let selected = choices
                            .iter()
                            .find(|choice| choice.value == *value)
                            .map_or("Unknown", |choice| choice.label);
                        let mut selection = None;

                        egui::ComboBox::from_id_salt((group.name, *label))
                            .selected_text(format!("{label}: {selected}"))
                            .show_ui(ui, |ui| {
                                for choice in *choices {
                                    if ui
                                        .selectable_label(*value == choice.value, choice.label)
                                        .clicked()
                                    {
                                        *value = choice.value;
                                        selection = Some(choice.value);
                                    }
                                }
                            });

                        if let Some(value) = selection {
                            sender
                                .send(Set(ParameterValue::Choice(*choice_parameter, value)))
                                .unwrap();
                        }
                    }
                }
            }
        });
    });
}

#[derive(Debug, Clone)]
struct DeviceState {
    floats: EnumMap<FloatParameters, f32>,
    bools: EnumMap<BoolParameters, bool>,
    uints: EnumMap<UIntParameters, u32>,
    choices: EnumMap<ChoiceParameters, u32>,
}

impl Default for DeviceState {
    fn default() -> Self {
        let mut state = Self {
            floats: Default::default(),
            bools: Default::default(),
            uints: Default::default(),
            choices: Default::default(),
        };
        state.floats[FloatParameters::Pan(0)] = 0.5;
        state.floats[FloatParameters::Pan(1)] = 0.5;
        state.floats[FloatParameters::ReverbSend(0)] = -96.0;
        state.floats[FloatParameters::ReverbSend(1)] = -96.0;
        for bus in 0..=2 {
            for source in 0..=5 {
                state.floats[FloatParameters::MixFader(bus, source.into())] = 0.0;
            }
        }
        state.choices[ChoiceParameters::MonitorSource()] = u32::MAX;
        state.choices[ChoiceParameters::ProcessingOrder(0)] = u32::MAX;
        state.choices[ChoiceParameters::ProcessingOrder(1)] = u32::MAX;
        state
    }
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
        ParameterValue::UInt(uint_param, value) => {
            state.uints[uint_param] = value;
        }
        ParameterValue::Choice(choice_param, value) => {
            state.choices[choice_param] = value;
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        // self.usb_updates
        //     .send(device_state::Command::Refresh())
        //     .unwrap();
        drain_usb_events(&self.usb_events, &mut self.device_state);

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let channel_linked = self.device_state.bools[BoolParameters::ChannelLink()];
                for group in self
                    .device_model
                    .groups
                    .iter()
                    .filter(|group| group.visibility.is_visible(channel_linked))
                {
                    parameter_group(ui, &mut self.device_state, &self.usb_updates, group);
                    ui.add_space(4.0);
                }
            });
        });

        thread::sleep(Duration::from_millis(14));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
}
