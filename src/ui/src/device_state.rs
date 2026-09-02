use crate::define_parameter_kind;
use crate::state::ParameterKind;
use presonus_revalator::protocol::payload::{JaSt, MixChannel};

#[derive(Debug, Clone)]
pub struct DeviceModel {
    pub groups: Vec<ParameterGroup>,
}

#[derive(Debug, Clone)]
pub struct ParameterGroup {
    pub name: &'static str,
    pub visibility: ParameterGroupVisibility,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterGroupVisibility {
    Always,
    Linked,
    Unlinked,
}

impl ParameterGroupVisibility {
    pub fn is_visible(self, channel_linked: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Linked => channel_linked,
            Self::Unlinked => !channel_linked,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Initialize(),
    Refresh(),
    RefreshMixers(),
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

    pub ChoiceParameter(u32) {}
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
        min: 0.0,
        max: 1.0,
    },
    HeadphonesGain() {
        min: 0.0,
        max: 1.0,
    },
    MonitorMix() {
        min: -1.0,
        max: 1.0,
    },
    Gain(_channel: u8) {
        min: 0.0,
        max: 60.0,
    },
    Pan(_channel: u8) {
        min: 0.0,
        max: 1.0,
    },
    ReverbSend(_channel: u8) {
        min: -96.0,
        max: 0.0,
    },
    MixFader(_bus: u8, _source: MixChannel) {
        min: -96.0,
        max: 10.0,
    }
}

impl FloatParameters {
    /// Whether this parameter is stored in decibels rather than a linear unit.
    pub fn is_decibels(self) -> bool {
        return false;
        matches!(
            self,
            Self::HeadphonesGain()
                | Self::Gain(_)
                | Self::ReverbSend(_)
                | Self::MixFader(_, _)
        )
    }

    pub fn is_percentage(self) -> bool {
        matches!(self, Self::MainGain())
    }

    pub fn is_reduction_meter(self) -> bool {
        matches!(self, Self::Gate(_) | Self::Compressor(_) | Self::Limiter(_))
    }
}

bool_parameters! {
    PresetEnabled(_channel: u8) {},
    LinkedPresetEnabled() {},
    PhantomPower(_channel: u8) {},
    Clipping(_channel: u8) {},
    ChannelLink() {},
    MixMono(_bus: u8) {}
}

u_int_parameters! {
    ActivityMask(_channel: u8) {},
    StatusFlags(_index: u8) {},
    EventFlag(_flag: u8) {}
}

choice_parameters! {
    MonitorSource() {},
    ProcessingOrder(_channel: u8) {}
}

#[derive(Debug, Clone, Copy)]
pub struct Choice {
    pub value: u32,
    pub label: &'static str,
}

const PROCESSING_ORDERS: &[Choice] = &[
    Choice {
        value: 0,
        label: "Compressor → EQ",
    },
    Choice {
        value: 1,
        label: "EQ → Compressor",
    },
];

const MONITOR_SOURCES: &[Choice] = &[
    Choice {
        value: 0,
        label: "Main",
    },
    Choice {
        value: 1,
        label: "Mix A",
    },
    Choice {
        value: 2,
        label: "Mix B",
    },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterValue {
    Float(FloatParameters, f32),
    Bool(BoolParameters, bool),
    UInt(UIntParameters, u32),
    Choice(ChoiceParameters, u32),
}

#[derive(Debug, Clone)]
pub enum Parameter {
    Knob(FloatParameters, &'static str),
    Fader(FloatParameters, &'static str),
    Meter(Vec<(FloatParameters, Option<BoolParameters>)>, &'static str),
    InvertedMeter(Vec<FloatParameters>, &'static str),
    UIntValue(UIntParameters, &'static str),
    Indicator(BoolParameters, &'static str),
    Button(BoolParameters, &'static str),
    Choice(ChoiceParameters, &'static str, &'static [Choice]),
}

pub fn device_model() -> DeviceModel {
    DeviceModel {
        groups: vec![
            ParameterGroup {
                name: "Channel 1",
                visibility: ParameterGroupVisibility::Unlinked,
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::ActivityMask(0), "Activity mask"),
                    Parameter::Button(BoolParameters::PresetEnabled(0), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(0), "48 V"),
                    Parameter::Knob(FloatParameters::Gain(0), "Gain"),
                    Parameter::Knob(FloatParameters::Pan(0), "Pan (L–C–R)"),
                    Parameter::Knob(FloatParameters::ReverbSend(0), "Reverb send (dB)"),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(0),
                        "Processing order",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Meter(vec![(FloatParameters::ChannelLevel(0), Some(BoolParameters::Clipping(0)))], "Level"),
                    Parameter::InvertedMeter(vec![FloatParameters::Gate(0)], "Gate"),
                    Parameter::InvertedMeter(vec![FloatParameters::Compressor(0)], "Compressor"),
                    Parameter::InvertedMeter(vec![FloatParameters::Limiter(0)], "Limiter"),
                ],
            },
            ParameterGroup {
                name: "Channel 2",
                visibility: ParameterGroupVisibility::Unlinked,
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::ActivityMask(1), "Activity mask"),
                    Parameter::Button(BoolParameters::PresetEnabled(1), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(1), "48 V"),
                    Parameter::Button(BoolParameters::ChannelLink(), "Channel link"),
                    Parameter::Knob(FloatParameters::Gain(1), "Gain"),
                    Parameter::Knob(FloatParameters::Pan(1), "Pan (L–C–R)"),
                    Parameter::Knob(FloatParameters::ReverbSend(1), "Reverb send (dB)"),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(1),
                        "Processing order",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Meter(vec![(FloatParameters::ChannelLevel(1), Some(BoolParameters::Clipping(1)))], "Level"),
                    Parameter::Meter(vec![(FloatParameters::Channel2Side(0), None), (FloatParameters::Channel2Side(1), None)], "Left"),
                    Parameter::InvertedMeter(vec![FloatParameters::Gate(1)], "Gate"),
                    Parameter::InvertedMeter(vec![FloatParameters::Compressor(1)], "Compressor"),
                    Parameter::InvertedMeter(vec![FloatParameters::Limiter(1)], "Limiter"),
                ],
            },
            ParameterGroup {
                name: "Channels 1+2 (linked)",
                visibility: ParameterGroupVisibility::Linked,
                parameters: vec![
                    Parameter::UIntValue(UIntParameters::ActivityMask(0), "Activity 1"),
                    Parameter::UIntValue(UIntParameters::ActivityMask(1), "Activity 2"),
                    Parameter::Button(BoolParameters::LinkedPresetEnabled(), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(0), "48 V 1"),
                    Parameter::Button(BoolParameters::PhantomPower(1), "48 V 2"),
                    Parameter::Button(BoolParameters::ChannelLink(), "Channel link"),
                    Parameter::Knob(FloatParameters::Gain(0), "Gain 1"),
                    Parameter::Knob(FloatParameters::Gain(1), "Gain 2"),
                    Parameter::Knob(FloatParameters::Pan(0), "Pan 1 (L–C–R)"),
                    Parameter::Knob(FloatParameters::Pan(1), "Pan 2 (L–C–R)"),
                    Parameter::Knob(FloatParameters::ReverbSend(0), "Reverb send 1 (dB)"),
                    Parameter::Knob(FloatParameters::ReverbSend(1), "Reverb send 2 (dB)"),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(0),
                        "Processing order 1",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(1),
                        "Processing order 2",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Meter(vec![(FloatParameters::Channel2Side(0), Some(BoolParameters::Clipping(0))), (FloatParameters::Channel2Side(1), Some(BoolParameters::Clipping(1)))], "Left"),
                    Parameter::InvertedMeter(vec![FloatParameters::Gate(0), FloatParameters::Gate(1)], "Gate 1"),
                    Parameter::InvertedMeter(vec![FloatParameters::Compressor(0), FloatParameters::Compressor(1)], "Compressor 1"),
                    Parameter::InvertedMeter(vec![FloatParameters::Limiter(0), FloatParameters::Limiter(1)], "Limiter 1"),
                ],
            },
            stereo_meter_group("Playback", FloatParameters::Playback),
            stereo_meter_group("Virtual A", FloatParameters::VirtualA),
            stereo_meter_group("Virtual B", FloatParameters::VirtualB),
            mix_group("Main", 0, FloatParameters::Main, true),
            mix_group("Mix A", 1, FloatParameters::MixA, false),
            mix_group("Mix B", 2, FloatParameters::MixB, false),
            ParameterGroup {
                name: "Monitor",
                visibility: ParameterGroupVisibility::Always,
                parameters: vec![
                    Parameter::Knob(FloatParameters::HeadphonesGain(), "Headphones output"),
                    Parameter::Knob(FloatParameters::MonitorMix(), "Blend"),
                    Parameter::Choice(ChoiceParameters::MonitorSource(), "Source", MONITOR_SOURCES),
                ],
            },
            ParameterGroup {
                name: "Raw flags",
                visibility: ParameterGroupVisibility::Always,
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

fn mix_group(
    name: &'static str,
    bus: u8,
    meter: fn(u8) -> FloatParameters,
    include_main_controls: bool,
) -> ParameterGroup {
    let mut parameters = vec![
        Parameter::Meter(vec![(meter(0), None), (meter(1), None)], "Output"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::Playback), "Playback"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::VirtualA), "Virtual A"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::VirtualB), "Virtual B"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::Channel1), "Channel 1"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::Channel2), "Channel 2"),
        Parameter::Fader(FloatParameters::MixFader(bus, MixChannel::Reverb), "Reverb"),
        Parameter::Button(BoolParameters::MixMono(bus), "Mono"),
    ];
    if include_main_controls {
        parameters.push(Parameter::Knob(FloatParameters::MainGain(), "Main output"));
    }

    ParameterGroup {
        name,
        visibility: ParameterGroupVisibility::Always,
        parameters,
    }
}

fn stereo_meter_group(name: &'static str, parameter: fn(u8) -> FloatParameters) -> ParameterGroup {
    ParameterGroup {
        name,
        visibility: ParameterGroupVisibility::Always,
        parameters: vec![
            Parameter::Meter(vec![(parameter(0), None), (parameter(1), None)], "Output"),
        ],
    }
}

pub fn jast_parameter_values(payload: JaSt) -> Vec<ParameterValue> {
    const CLIPPING_MASK: u32 = 0x0000_00c0;

    let channel_linked = payload.status_flags_1.channel_link();
    let channel_1_preset_enabled = !payload.status_flags_0.channel_1_preset_disabled();
    let channel_2_preset_enabled = !payload.status_flags_0.channel_2_preset_disabled();

    vec![
        float(FloatParameters::Playback(0), payload.playback_left),
        float(FloatParameters::Playback(1), payload.playback_right),
        float(FloatParameters::VirtualA(0), payload.virtual_a_left),
        float(FloatParameters::VirtualA(1), payload.virtual_a_right),
        float(FloatParameters::VirtualB(0), payload.virtual_b_left),
        float(FloatParameters::VirtualB(1), payload.virtual_b_right),
        float(FloatParameters::Channel2Side(0), payload.stereo_left),
        float(FloatParameters::ChannelLevel(0), payload.channel_1),
        float(FloatParameters::Channel2Side(1), payload.stereo_right),
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
        bool_value(
            BoolParameters::Clipping(0),
            payload.channel_1_activity & CLIPPING_MASK == CLIPPING_MASK,
        ),
        bool_value(
            BoolParameters::Clipping(1),
            payload.channel_2_activity & CLIPPING_MASK == CLIPPING_MASK,
        ),
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
        bool_value(BoolParameters::PresetEnabled(0), channel_1_preset_enabled),
        bool_value(BoolParameters::PresetEnabled(1), channel_2_preset_enabled),
        bool_value(
            BoolParameters::LinkedPresetEnabled(),
            channel_1_preset_enabled && channel_2_preset_enabled,
        ),
        bool_value(
            BoolParameters::PhantomPower(0),
            payload.channel_1_flags.enabled_48v(),
        ),
        bool_value(
            BoolParameters::PhantomPower(1),
            payload.channel_2_flags.enabled_48v(),
        ),
        bool_value(BoolParameters::ChannelLink(), channel_linked),
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
        payload.stereo_right = 0.5;
        payload.channel_1_activity = 1;
        payload.channel_2_activity = 0x0000_00c0;
        payload.status_flags_0.set_channel_1_preset_disabled(false);
        payload.status_flags_0.set_channel_2_preset_disabled(true);
        payload.status_flags_1.set_channel_link(true);
        payload.status_flags_2 = 0x3456;
        payload.channel_1_flags.set_enabled_48v(true);
        payload.channel_2_flags.set_enabled_48v(true);
        payload.monitor_mix = 0.25;
        payload.channel_2_gain = 42.0;
        payload.flag_3 = 7;

        let values = jast_parameter_values(payload);

        assert_eq!(values.len(), 44);
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Playback(0), 0.25, )));
        assert!(values.contains(&ParameterValue::Float(
            FloatParameters::Channel2Side(1),
            0.5,
        )));
        assert!(values.contains(&ParameterValue::UInt(UIntParameters::ActivityMask(0), 1, )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::PresetEnabled(0),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::PresetEnabled(1),
            false,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::LinkedPresetEnabled(),
            false,
        )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::Clipping(0), false, )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::Clipping(1), true, )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::PhantomPower(0), true, )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::PhantomPower(1), true, )));
        assert!(values.contains(&ParameterValue::Bool(BoolParameters::ChannelLink(), true, )));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::MonitorMix(), 0.25, )));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Gain(1), 42.0, )));
        assert!(values.contains(&ParameterValue::UInt(UIntParameters::EventFlag(3), 7, )));
    }

    #[test]
    fn exposes_mono_buttons_and_output_knobs() {
        let parameters = device_model()
            .groups
            .into_iter()
            .flat_map(|group| group.parameters)
            .collect::<Vec<_>>();

        assert_eq!(
            parameters
                .iter()
                .filter(|parameter| matches!(
                    parameter,
                    Parameter::Button(BoolParameters::MixMono(_), _)
                ))
                .count(),
            3
        );
        for expected in [
            FloatParameters::HeadphonesGain(),
            FloatParameters::MainGain(),
            FloatParameters::MonitorMix(),
        ] {
            assert!(parameters.iter().any(
                |parameter| matches!(parameter, Parameter::Knob(actual, _) if *actual == expected)
            ));
        }
    }

    #[test]
    fn assigns_control_units_and_meter_direction() {
        for parameter in [
            FloatParameters::Gain(0),
            FloatParameters::ReverbSend(0),
            FloatParameters::HeadphonesGain(),
            FloatParameters::MixFader(0, MixChannel::Playback),
        ] {
            assert!(parameter.is_decibels());
        }
        assert!(!FloatParameters::MainGain().is_decibels());
        assert!(FloatParameters::MainGain().is_percentage());
        assert_eq!(FloatParameters::MainGain().spec().min, 0.0);
        assert_eq!(FloatParameters::MainGain().spec().max, 1.0);

        assert!(FloatParameters::Compressor(0).is_reduction_meter());
        assert!(FloatParameters::Gate(0).is_reduction_meter());
        assert!(FloatParameters::Limiter(0).is_reduction_meter());
        assert!(!FloatParameters::ChannelLevel(0).is_reduction_meter());

        assert_eq!(
            FloatParameters::MixFader(0, MixChannel::Playback).spec().min,
            -96.0
        );
    }

    #[test]
    fn exposes_six_faders_for_each_mix() {
        let faders = device_model()
            .groups
            .into_iter()
            .flat_map(|group| group.parameters)
            .filter(|parameter| matches!(parameter, Parameter::Fader(_, _)))
            .count();

        assert_eq!(faders, 18);
    }
}
