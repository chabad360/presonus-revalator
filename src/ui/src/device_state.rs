use crate::define_parameter_kind;
use crate::state::ParameterKind;
use presonus_revalator::protocol::payload::JaSt;

#[derive(Debug, Clone)]
pub struct DeviceModel {
    pub groups: Vec<ParameterGroup>,
}

#[derive(Debug, Clone)]
pub struct ParameterGroup {
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

    pub UIntParameter(u32) {}
}

float_parameters! {
    Playback(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    VirtualA(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    VirtualB(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    ChannelLevel(_channel: u8) {
        min: 0.0,
        max: 1.0,
    },
    Channel2Side(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    Main(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    MixA(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    MixB(_side: u8) {
        min: 0.0,
        max: 1.0,
    },
    Gate(_channel: u8) {
        min: 0.0,
        max: 1.0,
    },
    Compressor(_channel: u8) {
        min: 0.0,
        max: 1.0,
    },
    Limiter(_channel: u8) {
        min: 0.0,
        max: 1.0,
    },
    MainGain() {
        min: f32::MIN,
        max: 0.0,
    },
    HeadphonesGain() {
        min: f32::MIN,
        max: 0.0,
    },
    MonitorMix() {
        min: -1.0,
        max: 1.0,
    },
    Gain(_channel: u8) {
        min: 0.0,
        max: 60.0,
    }
}

bool_parameters! {
    PresetEnabled(_channel: u8) {},
    PhantomPower(_channel: u8) {},
    StereoInput() {}
}

u_int_parameters! {
    ActivityMask(_channel: u8) {},
    StatusFlags(_index: u8) {},
    EventFlag(_flag: u8) {}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterValue {
    Float(FloatParameters, f32),
    Bool(BoolParameters, bool),
    UInt(UIntParameters, u32),
}

#[derive(Debug, Clone, Copy)]
pub enum Parameter {
    Knob(FloatParameters, &'static str),
    Meter(FloatParameters, &'static str),
    Value(FloatParameters, &'static str),
    UIntValue(UIntParameters, &'static str),
    BoolValue(BoolParameters, &'static str),
    Button(BoolParameters, &'static str),
}

pub fn device_model() -> DeviceModel {
    DeviceModel {
        groups: vec![
            ParameterGroup {
                name: "Channel 1",
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::ActivityMask(0), "Activity mask"),
                    Parameter::Button(BoolParameters::PresetEnabled(0), "Preset"),
                    Parameter::BoolValue(BoolParameters::PhantomPower(0), "48 V"),
                    Parameter::Knob(FloatParameters::Gain(0), "Gain"),
                    Parameter::Meter(FloatParameters::ChannelLevel(0), "Level"),
                    Parameter::Meter(FloatParameters::Gate(0), "Gate"),
                    Parameter::Meter(FloatParameters::Compressor(0), "Compressor"),
                    Parameter::Meter(FloatParameters::Limiter(0), "Limiter"),
                ],
            },
            ParameterGroup {
                name: "Channel 2",
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::ActivityMask(1), "Activity mask"),
                    Parameter::Button(BoolParameters::PresetEnabled(1), "Preset"),
                    Parameter::BoolValue(BoolParameters::PhantomPower(1), "48 V"),
                    Parameter::BoolValue(BoolParameters::StereoInput(), "Stereo input"),
                    Parameter::Knob(FloatParameters::Gain(1), "Gain"),
                    Parameter::Meter(FloatParameters::ChannelLevel(1), "Level"),
                    Parameter::Meter(FloatParameters::Channel2Side(0), "Left"),
                    Parameter::Meter(FloatParameters::Channel2Side(1), "Right"),
                    Parameter::Meter(FloatParameters::Gate(1), "Gate"),
                    Parameter::Meter(FloatParameters::Compressor(1), "Compressor"),
                    Parameter::Meter(FloatParameters::Limiter(1), "Limiter"),
                ],
            },
            stereo_meter_group("Playback", FloatParameters::Playback),
            stereo_meter_group("Virtual A", FloatParameters::VirtualA),
            stereo_meter_group("Virtual B", FloatParameters::VirtualB),
            ParameterGroup {
                name: "Main",
                parameters: vec![
                    Parameter::Meter(FloatParameters::Main(0), "Left"),
                    Parameter::Meter(FloatParameters::Main(1), "Right"),
                    Parameter::Value(FloatParameters::MainGain(), "Gain"),
                ],
            },
            stereo_meter_group("Mix A", FloatParameters::MixA),
            stereo_meter_group("Mix B", FloatParameters::MixB),
            ParameterGroup {
                name: "Monitor",
                parameters: vec![
                    Parameter::Value(FloatParameters::HeadphonesGain(), "Headphones gain"),
                    Parameter::Value(FloatParameters::MonitorMix(), "Monitor mix"),
                ],
            },
            ParameterGroup {
                name: "Raw flags",
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::StatusFlags(0), "Status byte 0"),
                    Parameter::UIntValue(UIntParameters::StatusFlags(1), "Status byte 1"),
                    Parameter::UIntValue(UIntParameters::StatusFlags(2), "Status word 2"),
                    Parameter::UIntValue(UIntParameters::EventFlag(0), "Event byte 0"),
                    Parameter::UIntValue(UIntParameters::EventFlag(1), "Event byte 1"),
                    Parameter::UIntValue(UIntParameters::EventFlag(2), "Event byte 2"),
                    Parameter::UIntValue(UIntParameters::EventFlag(3), "Event byte 3"),
                ],
            },
        ],
    }
}

fn stereo_meter_group(name: &'static str, parameter: fn(u8) -> FloatParameters) -> ParameterGroup {
    ParameterGroup {
        name,
        parameters: vec![
            Parameter::Meter(parameter(0), "Left"),
            Parameter::Meter(parameter(1), "Right"),
        ],
    }
}

pub fn jast_parameter_values(payload: JaSt) -> Vec<ParameterValue> {
    vec![
        float(FloatParameters::Playback(0), payload.playback_left),
        float(FloatParameters::Playback(1), payload.playback_right),
        float(FloatParameters::VirtualA(0), payload.virtual_a_left),
        float(FloatParameters::VirtualA(1), payload.virtual_a_right),
        float(FloatParameters::VirtualB(0), payload.virtual_b_left),
        float(FloatParameters::VirtualB(1), payload.virtual_b_right),
        float(FloatParameters::Channel2Side(0), payload.channel_2_left),
        float(FloatParameters::ChannelLevel(0), payload.channel_1),
        float(FloatParameters::Channel2Side(1), payload.channel_2_right),
        float(FloatParameters::ChannelLevel(1), payload.channel_2),
        float(FloatParameters::Main(0), payload.main_left),
        float(FloatParameters::Main(1), payload.main_right),
        float(FloatParameters::MixA(0), payload.mix_a_left),
        float(FloatParameters::MixA(1), payload.mix_a_right),
        float(FloatParameters::MixB(0), payload.mix_b_left),
        float(FloatParameters::MixB(1), payload.mix_b_right),
        float(FloatParameters::Gate(0), payload.channel_1_gate),
        float(FloatParameters::Compressor(0), payload.channel_1_comp),
        float(FloatParameters::Limiter(0), payload.channel_1_limiter),
        float(FloatParameters::Gate(1), payload.channel_2_gate),
        float(FloatParameters::Compressor(1), payload.channel_2_comp),
        float(FloatParameters::Limiter(1), payload.channel_2_limiter),
        uint(UIntParameters::ActivityMask(0), payload.channel_1_activity),
        uint(UIntParameters::ActivityMask(1), payload.channel_2_activity),
        uint(
            UIntParameters::StatusFlags(0),
            payload.status_flags_0.into_bytes()[0].into(),
        ),
        uint(
            UIntParameters::StatusFlags(1),
            payload.status_flags_1.into_bytes()[0].into(),
        ),
        uint(
            UIntParameters::StatusFlags(2),
            payload.status_flags_2.into(),
        ),
        bool_value(
            BoolParameters::PresetEnabled(0),
            !payload.status_flags_0.channel_1_preset_disabled(),
        ),
        bool_value(
            BoolParameters::PresetEnabled(1),
            !payload.status_flags_0.channel_2_preset_disabled(),
        ),
        bool_value(
            BoolParameters::PhantomPower(0),
            payload.channel_1_flags.enabled_48v(),
        ),
        bool_value(
            BoolParameters::PhantomPower(1),
            payload.channel_2_flags.enabled_48v(),
        ),
        bool_value(
            BoolParameters::StereoInput(),
            payload.status_flags_1.stereo_input(),
        ),
        float(FloatParameters::MainGain(), payload.main_gain),
        float(FloatParameters::HeadphonesGain(), payload.gain_headphones),
        float(FloatParameters::MonitorMix(), payload.monitor_mix),
        float(FloatParameters::Gain(0), payload.channel_1_gain),
        float(FloatParameters::Gain(1), payload.channel_2_gain),
        uint(
            UIntParameters::EventFlag(0),
            payload.channel_1_flags.into_bytes()[0].into(),
        ),
        uint(
            UIntParameters::EventFlag(1),
            payload.channel_2_flags.into_bytes()[0].into(),
        ),
        uint(UIntParameters::EventFlag(2), payload.flag_2.into()),
        uint(UIntParameters::EventFlag(3), payload.flag_3.into()),
    ]
}

fn float(parameter: FloatParameters, value: f32) -> ParameterValue {
    ParameterValue::Float(parameter, value)
}

fn bool_value(parameter: BoolParameters, value: bool) -> ParameterValue {
    ParameterValue::Bool(parameter, value)
}

fn uint(parameter: UIntParameters, value: u32) -> ParameterValue {
    ParameterValue::UInt(parameter, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use std::io::Cursor;

    #[test]
    fn maps_representative_jast_values() {
        let mut payload = JaSt::read_le(&mut Cursor::new([0_u8; 212])).unwrap();
        payload.playback_left = 0.25;
        payload.channel_2_right = 0.5;
        payload.channel_1_activity = 1;
        payload.status_flags_0.set_channel_1_preset_disabled(false);
        payload.status_flags_0.set_channel_2_preset_disabled(true);
        payload.status_flags_1.set_stereo_input(true);
        payload.status_flags_2 = 0x3456;
        payload.channel_1_flags.set_enabled_48v(true);
        payload.monitor_mix = 0.25;
        payload.channel_2_gain = 42.0;
        payload.flag_3 = 7;

        let values = jast_parameter_values(payload);

        assert_eq!(values.len(), 41);
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Playback(0), 0.25,)));
        assert!(values.contains(&ParameterValue::Float(
            FloatParameters::Channel2Side(1),
            0.5,
        )));
        assert!(values.contains(&ParameterValue::UInt(UIntParameters::ActivityMask(0), 1,)));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::PresetEnabled(0),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::PresetEnabled(1),
            false,
        )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::PhantomPower(0), true,)));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::StereoInput(), true,)));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::MonitorMix(), 0.25,)));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Gain(1), 42.0,)));
        assert!(values.contains(&ParameterValue::UInt(UIntParameters::EventFlag(3), 7,)));
    }
}
