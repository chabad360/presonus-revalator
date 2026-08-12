use crate::define_parameter_kind;
use crate::state::ParameterKind;

#[derive(Debug, Clone)]
pub struct DeviceModel {
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: &'static str,
    pub parameters: Vec<Parameter>,
}


#[derive(Debug, Clone)]
pub enum Command {
    Refresh(),
    Set(ParameterValue),
    SetMany(Vec<ParameterValue>),
}

#[derive(Debug, Clone)]
pub enum State {
    Update(ParameterValue),
    UpdateMany(Vec<ParameterValue>),

    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatWidget {
    Meter,
    Knob,
}

define_parameter_kind! {
    $;

    pub struct float_parameter(f32) {
        pub min: f32,
        pub max: f32,
    }
}

float_parameter! {
    pub enum FloatParameter {
        Gain(_channel: u8) {
            min: 0f32,
            max: 64f32,
        },
        Meter(_channel: u8) {
            min: f32::MIN,
            max: 0f32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolWidget {
    Toggle,

    Button,
}

define_parameter_kind! {
    $;

    pub struct bool_parameter(bool) {}
}

bool_parameter! {
    pub enum BoolParameter {
        Mute(_channel: u8) {}
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ParameterValue {
    Float(FloatParameter, f32),
    Bool(BoolParameter, bool),
}

#[derive(Debug, Clone, Copy)]
pub enum Parameter {
    Knob(FloatParameter, &'static str),
    Meter(FloatParameter, &'static str),
    Button(BoolParameter, &'static str),
}

pub fn device_model() -> DeviceModel {
    DeviceModel {
        channels: vec![Channel {
            name: "Channel 1",
            parameters: vec![
                Parameter::Button(BoolParameter::Mute(0), "Mute"),
                Parameter::Knob(FloatParameter::Gain(0), "Gain"),
                Parameter::Meter(FloatParameter::Meter(0), "Meter"),
            ],
        }],
    }
}
