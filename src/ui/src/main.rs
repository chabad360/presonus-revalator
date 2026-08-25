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
    BoolParameters, FloatParameters, Parameter, ParameterGroup, ParameterValue, State,
    UIntParameters, device_model,
};
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
                        if ui
                            .add(
                                Slider::new(
                                    &mut device_state.floats[*float_parameter],
                                    spec.min..=spec.max,
                                )
                                .text(*label),
                            )
                            .changed()
                        {
                            sender
                                .send(Set(ParameterValue::Float(
                                    *float_parameter,
                                    device_state.floats[*float_parameter],
                                )))
                                .unwrap();
                        }
                    }
                    Parameter::Meter(float_parameter, label) => {
                        let value = device_state.floats[*float_parameter];
                        let dbfs = meter_level_to_dbfs(value);
                        ui.add(
                            egui::ProgressBar::new(meter_fill(value))
                                .text(format!("{label}: {dbfs:.1} dBFS")),
                        );
                    }
                    Parameter::Value(float_parameter, label) => {
                        let value = device_state.floats[*float_parameter];
                        ui.label(format!("{label}: {value:.2}"));
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
                }
            }
        });
    });
}

fn meter_fill(level: f32) -> f32 {
    if level.is_nan() {
        return 0.0;
    }

    level.clamp(0.0, 1.0)
}

fn meter_level_to_dbfs(level: f32) -> f32 {
    if level <= 0.0 || level.is_nan() {
        return f32::NEG_INFINITY;
    }

    20.0 * level.log10()
}

#[derive(Debug, Clone, Default)]
struct DeviceState {
    floats: EnumMap<FloatParameters, f32>,
    bools: EnumMap<BoolParameters, bool>,
    uints: EnumMap<UIntParameters, u32>,
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
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests {
    use super::{meter_fill, meter_level_to_dbfs};

    #[test]
    fn renders_linear_meter_level_with_dbfs_label() {
        assert_eq!(meter_fill(0.5), 0.5);
        assert!((meter_level_to_dbfs(0.5) - -6.0206).abs() < 0.0001);
        assert_eq!(meter_fill(0.0), 0.0);
        assert_eq!(meter_level_to_dbfs(0.0), f32::NEG_INFINITY);
        assert_eq!(meter_fill(f32::NAN), 0.0);
        assert_eq!(meter_level_to_dbfs(f32::NAN), f32::NEG_INFINITY);
    }
}
