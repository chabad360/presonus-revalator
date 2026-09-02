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
    device_model, BoolParameters, ChoiceParameters, FloatParameters, MeterInput, Parameter,
    ParameterGroup, ParameterValue, State, UIntParameters, ValueDisplay,
};
use eframe::{egui, Frame};
use egui_audio_controls::{Fader, Knob, LevelMeter};
use enum_map::EnumMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const MIN_GROUP_WIDTH: f32 = 800.0;
const COMPACT_GROUP_WIDTH: f32 = 120.0;

#[derive(Clone, Copy)]
struct GroupLayout {
    min_width: f32,
    expands: bool,
}

fn group_layout(group: &ParameterGroup) -> GroupLayout {
    match group.parameters.as_slice() {
        [Parameter::Meter(_)] => GroupLayout {
            min_width: COMPACT_GROUP_WIDTH,
            expands: false,
        },
        _ => GroupLayout {
            min_width: MIN_GROUP_WIDTH,
            expands: true,
        },
    }
}

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

fn configure_knob_display(knob: Knob, display: ValueDisplay) -> Knob {
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

fn configure_fader_display(fader: Fader, display: ValueDisplay) -> Fader {
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
        ui.set_width(ui.available_width());
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
                let available_width = ui.available_width();
                let horizontal_spacing = ui.spacing().item_spacing.x;
                let visible_groups = self
                    .device_model
                    .groups
                    .iter()
                    .enumerate()
                    .filter_map(|(index, group)| {
                        group.visibility.is_visible(channel_linked).then_some(index)
                    })
                    .collect::<Vec<_>>();

                let mut rows = Vec::new();
                let mut current_row = Vec::new();
                let mut current_width = 0.0;

                for group_index in visible_groups {
                    let layout = group_layout(&self.device_model.groups[group_index]);
                    let next_width = if current_row.is_empty() {
                        layout.min_width
                    } else {
                        current_width + horizontal_spacing + layout.min_width
                    };

                    if !current_row.is_empty() && next_width > available_width {
                        rows.push(current_row);
                        current_row = vec![group_index];
                        current_width = layout.min_width;
                    } else {
                        current_row.push(group_index);
                        current_width = next_width;
                    }
                }

                if !current_row.is_empty() {
                    rows.push(current_row);
                }

                for row in rows {
                    let row_min_width = row
                        .iter()
                        .map(|&group_index| {
                            group_layout(&self.device_model.groups[group_index]).min_width
                        })
                        .sum::<f32>();
                    let expandable_count = row
                        .iter()
                        .filter(|&&group_index| {
                            group_layout(&self.device_model.groups[group_index]).expands
                        })
                        .count();
                    let extra_width = (available_width
                        - row_min_width
                        - horizontal_spacing * row.len().saturating_sub(1) as f32)
                        .max(0.0);
                    let extra_per_expanding_group = if expandable_count == 0 {
                        0.0
                    } else {
                        extra_width / expandable_count as f32
                    };

                    ui.horizontal_top(|ui| {
                        for &group_index in &row {
                            let group = &self.device_model.groups[group_index];
                            let layout = group_layout(group);
                            let group_width = layout.min_width
                                + if layout.expands {
                                extra_per_expanding_group
                            } else {
                                0.0
                            };
                            ui.allocate_ui_with_layout(
                                egui::vec2(group_width, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    parameter_group(
                                        ui,
                                        &mut self.device_state,
                                        &self.usb_updates,
                                        group,
                                    );
                                },
                            );
                        }
                    });
                }
            });
        });

        thread::sleep(Duration::from_millis(14));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
}
