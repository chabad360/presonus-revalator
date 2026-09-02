use crate::define_parameter_kind;
use crate::state::ParameterKind;
use egui_audio_controls::ValueScale;
use enum_map::Enum;
use presonus_revalator::protocol::payload::{JaSt, MixChannel};
use presonus_revalator::protocol::protocol::Bus;

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
    Reverb(_side: u8) {
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

bool_parameters! {
    PresetEnabled(_channel: u8) {},
    LinkedPresetEnabled() {},
    PhantomPower(_channel: u8) {},
    Clipping(_channel: u8) {},
    InputClipping(_channel: MixChannel, _side: u8) {},
    OutputClipping(_bus: Bus, _side: u8) {},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum Channel {
    Input(MixChannel),
    Output(Bus),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDisplay {
    Number {
        prefix: &'static str,
        suffix: &'static str,
        decimals: usize,
    },
    Percentage {
        decimals: usize,
    },
}

impl Default for ValueDisplay {
    fn default() -> Self {
        Self::Number {
            prefix: "",
            suffix: "",
            decimals: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KnobParameter {
    pub parameter: FloatParameters,
    pub label: &'static str,
    pub scale: ValueScale,
    pub display: ValueDisplay,
    pub notches: Vec<f32>,
    pub reset_value: Option<f32>,
}

impl KnobParameter {
    fn new(parameter: FloatParameters, label: &'static str) -> Self {
        Self {
            parameter,
            label,
            scale: ValueScale::Linear,
            display: ValueDisplay::default(),
            notches: Vec::new(),
            reset_value: None,
        }
    }

    fn decibels(mut self) -> Self {
        self.display = ValueDisplay::Number {
            prefix: "",
            suffix: " dB",
            decimals: 1,
        };
        self
    }

    fn percentage(mut self) -> Self {
        self.display = ValueDisplay::Percentage { decimals: 0 };
        self
    }

    fn notch(mut self, value: f32) -> Self {
        self.notches.push(value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct FaderParameter {
    pub parameter: FloatParameters,
    pub label: &'static str,
    pub display: ValueDisplay,
    pub decibel_taper: bool,
    pub notches: Vec<f32>,
    pub reset_value: Option<f32>,
}

impl FaderParameter {
    fn decibels(parameter: FloatParameters, label: &'static str) -> Self {
        Self {
            parameter,
            label,
            display: ValueDisplay::Number {
                prefix: "",
                suffix: " dB",
                decimals: 1,
            },
            decibel_taper: true,
            notches: vec![0.0],
            reset_value: Some(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterInput {
    LinearAmplitude,
    Decibels,
}

#[derive(Debug, Clone)]
pub struct MeterChannel {
    pub parameter: FloatParameters,
    pub clipping: Option<BoolParameters>,
    pub name: Option<&'static str>,
}

impl MeterChannel {
    fn new(parameter: FloatParameters) -> Self {
        Self {
            parameter,
            clipping: None,
            name: None,
        }
    }

    fn clipping(mut self, clipping: BoolParameters) -> Self {
        self.clipping = Some(clipping);
        self
    }

    fn named(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

#[derive(Debug, Clone)]
pub struct MeterParameter {
    pub channels: Vec<MeterChannel>,
    pub label: &'static str,
    pub input: MeterInput,
    pub inverted: bool,
    pub range_db: (f32, f32),
}

impl MeterParameter {
    fn level(channels: Vec<MeterChannel>, label: &'static str) -> Self {
        Self {
            channels,
            label,
            input: MeterInput::LinearAmplitude,
            inverted: false,
            range_db: (-60.0, 6.0),
        }
    }

    fn reduction(channels: Vec<MeterChannel>, label: &'static str) -> Self {
        Self {
            channels,
            label,
            input: MeterInput::LinearAmplitude,
            inverted: true,
            range_db: (-60.0, 0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Parameter {
    Knob(KnobParameter),
    Fader(FaderParameter),
    Meter(MeterParameter),
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
                    Parameter::Button(BoolParameters::PresetEnabled(0), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(0), "48 V"),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Gain(0), "Gain").decibels(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Pan(0), "Pan (L–C–R)").notch(0.5),
                    ),
                    Parameter::Fader(FaderParameter::decibels(
                        FloatParameters::ReverbSend(0),
                        "Reverb send",
                    )),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(0),
                        "Processing order",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Meter(MeterParameter::level(
                        vec![MeterChannel::new(FloatParameters::ChannelLevel(0))
                            .clipping(BoolParameters::InputClipping(MixChannel::Channel1, 0))],
                        "Level",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Gate(0))],
                        "Gate",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Compressor(0))],
                        "Compressor",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Limiter(0))],
                        "Limiter",
                    )),
                ],
            },
            ParameterGroup {
                name: "Channel 2",
                visibility: ParameterGroupVisibility::Unlinked,
                parameters: vec![
                    Parameter::Button(BoolParameters::PresetEnabled(1), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(1), "48 V"),
                    Parameter::Button(BoolParameters::ChannelLink(), "Channel link"),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Gain(1), "Gain").decibels(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Pan(1), "Pan (L–C–R)").notch(0.5),
                    ),
                    Parameter::Fader(FaderParameter::decibels(
                        FloatParameters::ReverbSend(1),
                        "Reverb send",
                    )),
                    Parameter::Choice(
                        ChoiceParameters::ProcessingOrder(1),
                        "Processing order",
                        PROCESSING_ORDERS,
                    ),
                    Parameter::Meter(MeterParameter::level(
                        vec![MeterChannel::new(FloatParameters::ChannelLevel(1))
                            .clipping(BoolParameters::InputClipping(MixChannel::Channel2, 0))],
                        "Level",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Gate(1))],
                        "Gate",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Compressor(1))],
                        "Compressor",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![MeterChannel::new(FloatParameters::Limiter(1))],
                        "Limiter",
                    )),
                ],
            },
            ParameterGroup {
                name: "Channels 1+2 (linked)",
                visibility: ParameterGroupVisibility::Linked,
                parameters: vec![
                    Parameter::Button(BoolParameters::LinkedPresetEnabled(), "Preset"),
                    Parameter::Button(BoolParameters::PhantomPower(0), "48 V 1"),
                    Parameter::Button(BoolParameters::PhantomPower(1), "48 V 2"),
                    Parameter::Button(BoolParameters::ChannelLink(), "Channel link"),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Gain(0), "Gain 1").decibels(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Gain(1), "Gain 2").decibels(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Pan(0), "Pan 1 (L–C–R)").notch(0.5),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::Pan(1), "Pan 2 (L–C–R)").notch(0.5),
                    ),
                    Parameter::Fader(FaderParameter::decibels(
                        FloatParameters::ReverbSend(0),
                        "Reverb send 1",
                    )),
                    Parameter::Fader(FaderParameter::decibels(
                        FloatParameters::ReverbSend(1),
                        "Reverb send 2",
                    )),
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
                    Parameter::Meter(MeterParameter::level(
                        vec![
                            MeterChannel::new(FloatParameters::Channel2Side(0))
                                .clipping(BoolParameters::InputClipping(MixChannel::Channel1, 0))
                                .named("Channel 1"),
                            MeterChannel::new(FloatParameters::Channel2Side(1))
                                .clipping(BoolParameters::InputClipping(MixChannel::Channel2, 0))
                                .named("Channel 2"),
                        ],
                        "Level",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![
                            MeterChannel::new(FloatParameters::Gate(0)).named("Channel 1"),
                            MeterChannel::new(FloatParameters::Gate(1)).named("Channel 2"),
                        ],
                        "Gate",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![
                            MeterChannel::new(FloatParameters::Compressor(0)).named("Channel 1"),
                            MeterChannel::new(FloatParameters::Compressor(1)).named("Channel 2"),
                        ],
                        "Compressor",
                    )),
                    Parameter::Meter(MeterParameter::reduction(
                        vec![
                            MeterChannel::new(FloatParameters::Limiter(0)).named("Channel 1"),
                            MeterChannel::new(FloatParameters::Limiter(1)).named("Channel 2"),
                        ],
                        "Limiter",
                    )),
                ],
            },
            stereo_meter_group("Playback", FloatParameters::Playback),
            stereo_meter_group("Virtual A", FloatParameters::VirtualA),
            stereo_meter_group("Virtual B", FloatParameters::VirtualB),
            stereo_meter_group("Reverb", FloatParameters::Reverb),
            mix_group("Main", 0, FloatParameters::Main),
            mix_group("Mix A", 1, FloatParameters::MixA),
            mix_group("Mix B", 2, FloatParameters::MixB),
            ParameterGroup {
                name: "Monitor",
                visibility: ParameterGroupVisibility::Always,
                parameters: vec![
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::MainGain(), "Speaker").percentage(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::HeadphonesGain(), "Headphones")
                            .percentage(),
                    ),
                    Parameter::Knob(
                        KnobParameter::new(FloatParameters::MonitorMix(), "Blend").notch(0.0),
                    ),
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
) -> ParameterGroup {
    let output_clipping = |side: u8| match bus {
        0 => BoolParameters::OutputClipping(Bus::MainMix, side),
        1 => BoolParameters::OutputClipping(Bus::MixA, side),
        2 => BoolParameters::OutputClipping(Bus::MixB, side),
        _ => panic!("unsupported mix bus: {bus}"),
    };

    let mut parameters = vec![
        Parameter::Meter(MeterParameter::level(
            vec![
                MeterChannel::new(meter(0))
                    .clipping(output_clipping(0))
                    .named("Left"),
                MeterChannel::new(meter(1))
                    .clipping(output_clipping(1))
                    .named("Right"),
            ],
            "Output",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::Playback),
            "Playback",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::VirtualA),
            "Virtual A",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::VirtualB),
            "Virtual B",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::Channel1),
            "Channel 1",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::Channel2),
            "Channel 2",
        )),
        Parameter::Fader(FaderParameter::decibels(
            FloatParameters::MixFader(bus, MixChannel::Reverb),
            "Reverb",
        )),
        Parameter::Button(BoolParameters::MixMono(bus), "Mono"),
    ];

    ParameterGroup {
        name,
        visibility: ParameterGroupVisibility::Always,
        parameters,
    }
}

fn stereo_meter_group(name: &'static str, parameter: fn(u8) -> FloatParameters) -> ParameterGroup {
    let input_clipping = |side: u8| match parameter(0) {
        FloatParameters::Playback(_) => BoolParameters::InputClipping(MixChannel::Playback, side),
        FloatParameters::VirtualA(_) => BoolParameters::InputClipping(MixChannel::VirtualA, side),
        FloatParameters::VirtualB(_) => BoolParameters::InputClipping(MixChannel::VirtualB, side),
        FloatParameters::Reverb(_) => BoolParameters::InputClipping(MixChannel::Reverb, side),
        _ => panic!("unsupported stereo meter parameter: {:?}", parameter(0)),
    };

    ParameterGroup {
        name,
        visibility: ParameterGroupVisibility::Always,
        parameters: vec![Parameter::Meter(MeterParameter::level(
            vec![
                MeterChannel::new(parameter(0))
                    .clipping(input_clipping(0))
                    .named("Left"),
                MeterChannel::new(parameter(1))
                    .clipping(input_clipping(1))
                    .named("Right"),
            ],
            "Output",
        ))],
    }
}

pub fn jast_parameter_values(payload: JaSt) -> Vec<ParameterValue> {
    const CLIPPING_MASK: u16 = 0x00c0;

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
        float(FloatParameters::Reverb(0), payload.reverb_l),
        float(FloatParameters::Reverb(1), payload.reverb_r),
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
        uint(
            UIntParameters::ActivityMask(0),
            payload.channel_1_activity.into(),
        ),
        uint(
            UIntParameters::ActivityMask(1),
            payload.channel_2_activity.into(),
        ),
        bool_value(
            BoolParameters::Clipping(0),
            (payload.channel_1_activity & CLIPPING_MASK) == CLIPPING_MASK,
        ),
        bool_value(
            BoolParameters::Clipping(1),
            (payload.channel_2_activity & CLIPPING_MASK) == CLIPPING_MASK,
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Playback, 0),
            payload.input_clipping.playback_l(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Playback, 1),
            payload.input_clipping.playback_r(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::VirtualA, 0),
            payload.input_clipping.virtual_a_l(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::VirtualA, 1),
            payload.input_clipping.virtual_a_r(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::VirtualB, 0),
            payload.input_clipping.virtual_b_l(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::VirtualB, 1),
            payload.input_clipping.virtual_b_r(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Channel1, 0),
            payload.input_clipping.channel_1(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Channel1, 1),
            payload.input_clipping.stereo_l(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Channel2, 0),
            payload.input_clipping.channel_2(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Channel2, 1),
            payload.input_clipping.stereo_r(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Reverb, 0),
            payload.input_clipping.reverb_l(),
        ),
        bool_value(
            BoolParameters::InputClipping(MixChannel::Reverb, 1),
            payload.input_clipping.reverb_r(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MainMix, 0),
            payload.output_clipping.main_l(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MainMix, 1),
            payload.output_clipping.main_r(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MixA, 0),
            payload.output_clipping.mix_a_l(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MixA, 1),
            payload.output_clipping.mix_a_r(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MixB, 0),
            payload.output_clipping.mix_b_l(),
        ),
        bool_value(
            BoolParameters::OutputClipping(Bus::MixB, 1),
            payload.output_clipping.mix_b_r(),
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
        payload.reverb_l = 0.125;
        payload.reverb_r = 0.75;
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
        payload.input_clipping.set_playback_l(true);
        payload.input_clipping.set_virtual_a_r(true);
        payload.input_clipping.set_channel_1(true);
        payload.input_clipping.set_stereo_l(true);
        payload.output_clipping.set_main_l(true);
        payload.output_clipping.set_mix_b_r(true);
        payload.flag_3 = 7;

        let values = jast_parameter_values(payload);

        assert_eq!(values.len(), 64);
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Playback(0), 0.25, )));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Reverb(0), 0.125, )));
        assert!(values.contains(&ParameterValue::Float(FloatParameters::Reverb(1), 0.75, )));
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
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::InputClipping(MixChannel::Playback, 0),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::InputClipping(MixChannel::VirtualA, 1),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::InputClipping(MixChannel::Channel1, 0),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::InputClipping(MixChannel::Channel1, 1),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::OutputClipping(Bus::MainMix, 0),
            true,
        )));
        assert!(values.contains(&ParameterValue::Bool(
            BoolParameters::OutputClipping(Bus::MixB, 1),
            true,
        )));
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
                |parameter| matches!(parameter, Parameter::Knob(spec) if spec.parameter == expected)
            ));
        }
        assert!(parameters.iter().any(|parameter| matches!(
            parameter,
            Parameter::Meter(spec)
                if spec.label == "Output"
                    && spec.channels.iter().any(|channel| channel.parameter == FloatParameters::Reverb(0))
                    && spec.channels.iter().any(|channel| channel.parameter == FloatParameters::Reverb(1))
        )));
    }

    #[test]
    fn stores_control_presentation_in_parameter_specs() {
        let parameters = device_model()
            .groups
            .into_iter()
            .flat_map(|group| group.parameters)
            .collect::<Vec<_>>();

        let gain = parameters.iter().find_map(|parameter| match parameter {
            Parameter::Knob(spec) if spec.parameter == FloatParameters::Gain(0) => Some(spec),
            _ => None,
        });
        assert!(matches!(
            gain.map(|spec| spec.display),
            Some(ValueDisplay::Number { suffix: " dB", .. })
        ));

        let main_output = parameters.iter().find_map(|parameter| match parameter {
            Parameter::Knob(spec) if spec.parameter == FloatParameters::MainGain() => Some(spec),
            _ => None,
        });
        assert!(matches!(
            main_output.map(|spec| spec.display),
            Some(ValueDisplay::Percentage { .. })
        ));

        let compressor = parameters.iter().find_map(|parameter| match parameter {
            Parameter::Meter(spec)
            if spec
                .channels
                .iter()
                .any(|channel| channel.parameter == FloatParameters::Compressor(0)) =>
                {
                    Some(spec)
                }
            _ => None,
        });
        assert!(matches!(
            compressor.map(|spec| (spec.input, spec.inverted)),
            Some((MeterInput::Decibels, true))
        ));

        assert_eq!(
            FloatParameters::MixFader(0, MixChannel::Playback)
                .spec()
                .min,
            -96.0
        );
    }

    #[test]
    fn exposes_six_faders_for_each_mix() {
        let faders = device_model()
            .groups
            .into_iter()
            .flat_map(|group| group.parameters)
            .filter(|parameter| matches!(parameter, Parameter::Fader(_)))
            .count();

        assert_eq!(faders, 22);
    }
}
