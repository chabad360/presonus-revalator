use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct DeviceModel {
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: &'static str,
    pub parameters: Vec<Parameter>,
}

pub type Id = &'static str;

#[derive(Debug, Clone)]
pub struct Meter {
    pub id: Id,
    pub range: RangeInclusive<f32>,
}
#[derive(Debug, Clone)]
pub struct Button {
    pub id: Id,
    pub label: &'static str,
}
#[derive(Debug, Clone)]
pub struct Knob {
    pub id: Id,
    pub range: RangeInclusive<f32>,
}

#[derive(Debug, Clone)]
pub enum Parameter {
    Meter(Meter),
    Button(Button),
    Knob(Knob),
}

#[derive(Debug, Clone)]
pub enum ParameterValue {
    Meter(Meter, f32),
    Button(Button, bool),
    Knob(Knob, f32),
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

pub fn device_model() -> DeviceModel {
    DeviceModel {
        channels: vec![Channel {
            name: "Channel 1",
            parameters: vec![
                Parameter::Button(Button {
                    id: "mute_1",
                    label: "Mute",
                }),
                Parameter::Knob(Knob {
                    id: "gain_1",
                    range: -0.0..=60.0,
                }),
                Parameter::Meter(Meter {
                    id: "meter_1",
                    range: 0.0..=1.0,
                }),
            ],
        }],
    }
}
