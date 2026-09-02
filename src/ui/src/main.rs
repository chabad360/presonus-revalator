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
    BoolParameters, ChoiceParameters, FloatParameters, MeterInput, Parameter, ParameterGroup,
    ParameterValue, State, UIntParameters, ValueDisplay, device_model,
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

fn configure_knob_display<'a>(knob: Knob<'a>, display: ValueDisplay) -> Knob<'a> {
    match display {
        ValueDisplay::Number {
            prefix,
            suffix,
            decimals,
        } => knob.prefix(prefix).suffix(suffix).decimals(decimals),
        ValueDisplay::Percentage { decimals } => knob
            .suffix("%")
            .decimals(decimals)
            .custom_formatter(move |value, _| format!("{:.*}", decimals, value * 100.0))
            .custom_parser(|text| {
                text.trim()
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .map(|value| value / 100.0)
            }),
    }
}

fn configure_fader_display<'a>(fader: Fader<'a>, display: ValueDisplay) -> Fader<'a> {
    match display {
        ValueDisplay::Number {
            prefix,
            suffix,
            decimals,
        } => fader.prefix(prefix).suffix(suffix).decimals(decimals),
        ValueDisplay::Percentage { decimals } => fader
            .suffix("%")
            .decimals(decimals)
            .custom_formatter(move |value, _| format!("{:.*}", decimals, value * 100.0))
            .custom_parser(|text| {
                text.trim()
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .map(|value| value / 100.0)
            }),
    }
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
                    Parameter::Knob(control) => {
                        let range = control.parameter.spec();
                        let mut knob = Knob::new(
                            &mut device_state.floats[control.parameter],
                            range.min..=range.max,
                        )
                        .scale(control.scale)
                        .notches(control.notches.iter().copied())
                        .text(control.label);
                        if let Some(reset_value) = control.reset_value {
                            knob = knob.reset_value(reset_value);
                        }
                        knob = configure_knob_display(knob, control.display);

                        if ui.add(knob).changed() {
                            sender
                                .send(Set(ParameterValue::Float(
                                    control.parameter,
                                    device_state.floats[control.parameter],
                                )))
                                .unwrap();
                        }
                    }
                    Parameter::Fader(control) => {
                        let range = control.parameter.spec();
                        let mut fader = Fader::new(
                            &mut device_state.floats[control.parameter],
                            range.min..=range.max,
                        )
                        .notches(control.notches.iter().copied())
                        .text(control.label);
                        if control.decibel_taper {
                            fader = fader.decibels();
                        }
                        if let Some(reset_value) = control.reset_value {
                            fader = fader.reset_value(reset_value);
                        }
                        fader = configure_fader_display(fader, control.display);

                        if ui.add(fader).changed() {
                            sender
                                .send(Set(ParameterValue::Float(
                                    control.parameter,
                                    device_state.floats[control.parameter],
                                )))
                                .unwrap();
                        }
                    }
                    Parameter::Meter(meter_spec) => {
                        let values = meter_spec
                            .channels
                            .iter()
                            .map(|channel| device_state.floats[channel.parameter])
                            .collect::<Vec<_>>();
                        let clipping = meter_spec
                            .channels
                            .iter()
                            .map(|channel| {
                                channel
                                    .clipping
                                    .map(|parameter| device_state.bools[parameter])
                            })
                            .collect::<Vec<_>>();
                        let names = meter_spec
                            .channels
                            .iter()
                            .map(|channel| channel.name)
                            .collect::<Vec<_>>();
                        let mut meter = match meter_spec.input {
                            MeterInput::LinearAmplitude => LevelMeter::new(values),
                            MeterInput::Decibels => LevelMeter::from_db(values),
                        }
                        .range_db(meter_spec.range_db.0..=meter_spec.range_db.1)
                        .optional_channel_clipping(clipping)
                        .optional_channel_names(names)
                        .text(meter_spec.label);
                        if meter_spec.inverted {
                            meter = meter.inverted();
                        }
                        ui.add(meter);
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
