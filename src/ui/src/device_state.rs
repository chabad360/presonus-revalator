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

define_parameter_kind! {
    $;

    pub FloatParameter(f32) {
        pub min: f32,
        pub max: f32,
    }

    pub BoolParameter(bool) {}
}

float_parameters! {
    Gain(_channel: u8) {
        min: 0f32,
        max: 64f32,
    },
    Meter(_channel: u8) {
        min: f32::MIN,
        max: 0f32,
    }
}

bool_parameters! {
    Mute(_channel: u8) {}
}

#[derive(Debug, Clone, Copy)]
pub enum ParameterValue {
    Float(FloatParameters, f32),
    Bool(BoolParameters, bool),
}

#[derive(Debug, Clone, Copy)]
pub enum Parameter {
    Knob(FloatParameters, &'static str),
    Meter(FloatParameters, &'static str),
    Button(BoolParameters, &'static str),
}

pub fn device_model() -> DeviceModel {
    DeviceModel {
        channels: vec![Channel {
            name: "Channel 1",
            parameters: vec![
                Parameter::Button(BoolParameters::Mute(0), "Mute"),
                Parameter::Knob(FloatParameters::Gain(0), "Gain"),
                Parameter::Meter(FloatParameters::Meter(0), "Meter"),
            ],
        }],
    }
}
