use crate::protocol::protocol::{Bus, Channel};
use binrw::{BinRead, BinResult, BinWrite};
use enum_map::Enum;
use modular_bitfield::prelude::*;
use serde::{Deserialize, Serialize};
use serde_ub_json::from_bytes;
use serde_ubj::to_vec;
use std::io::{Error, ErrorKind};

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum ApplPayload {
    #[brw(
        magic = b"tSaJ\xdc\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\xe1\x01"
    )]
    AskJaSt,

    #[brw(magic = b"tSaJ\xdc\0\0\0")]
    JaSt(JaSt),

    #[brw(magic = b"araP\x14\0\0\0")]
    Para {
        channel: Channel,
        data: ApplParaParameter,
    },

    #[brw(magic = b"iraP\x14\0\0\0")]
    Pari {
        channel: Channel,
        data: ApplPariParameter,
    },

    #[brw(magic = b"PmeM")]
    MemP {
        size: u32,
        store: Store,
        page_0: u32,
        page_1: u32,
        version: u32,
        header: u32,

        #[br(count = size - 29)]
        data: Vec<u8>,
        // #[br(pad_before(1), args{ size: size - 29 })]
        // parsed: MemoryData,
    },

    #[brw(magic = b"PmeM")]
    GetMemP {
        size: u32,
        store: Store,
        page_0: u32,
        page_1: u32,
        unknown: u32,
    },

    #[brw(magic = b"fdqB\x24\0\0\0")]
    Bqdf {
        unknown: u32,
        idx: u32,
        coefficients: [f32; 4],
    },

    #[brw(magic = b"ffLM")]
    MLff {
        size: u32,
        unknown: u32,
        #[br(count = (size - 12)/4)]
        data: Vec<f32>,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum DefaultPayload {
    #[brw(magic = b"araP\x14\0\0\0")]
    Para {
        channel: Channel,
        data: DefaultParaParameter,
    },

    #[brw(magic = b"iraP\x14\0\0\0")]
    Pari {
        channel: Channel,
        data: DefaultPariParameter,
    },

    #[brw(magic = b"PmeM")]
    MemP {
        size: u32,
        store: Store,
        page_0: u32,
        page_1: u32,
        version: u32,
        header: u32,

        #[br(count = size - 29)]
        data: Vec<u8>,
        // #[br(pad_before(1), args{ size: size - 29 })]
        // parsed: MemoryData,
    },

    #[brw(magic = b"PmeM")]
    GetMemP {
        size: u32,
        store: Store,
        page_0: u32,
        page_1: u32,
        unknown: u32,
    },

    #[brw(magic = b"fdqB\x24\0\0\0")]
    Bqdf {
        unknown: u32,
        idx: u32,
        coefficients: [f32; 4],
    },

    #[brw(magic = b"ffLM")]
    MLff {
        size: u32,
        unknown: u32,
        #[br(count = (size - 12)/4)]
        data: Vec<f32>,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum UnknownPayload {
    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum FilterPayload {
    #[brw(magic = b"fdBM\xf4\x01\0\0")]
    MBdf {
        unknown_0: u32,
        unknown_1: u32,
        sample_rate_coefficients: [BiquadRate; 4],
        // #[br(count = size - (4 * std::mem::size_of::<BiquadRate>() as u32) - 8 - 8)]
        extra: [u32; 388 / 4],
    },

    #[brw(magic = b"fdqB\x24\0\0\0")]
    Bqdf {
        unknown: u32,
        idx: u32,
        coefficients: [f32; 4],
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum ReverbPayload {
    #[brw(magic = b"bvrv\x38\0\0\0")]
    Reverb {
        unknown_0: u32,
        unknown_1: u32,
        unknown_2: f32,
        unknown_3: u32,
        pre_delay_sec: f32,
        size: f32,
        hpf: f32,
        coefficients: [f32; 5],
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum OptPayload {
    #[brw(magic = b"araP\x14\0\0\0")]
    Para {
        channel: Channel,
        data: OptParaParameter,
    },

    #[brw(magic = b"iraP\x14\0\0\0")]
    Pari {
        channel: Channel,
        data: OptPariParameter,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum BusPayload {
    #[brw(magic = b"araP\x14\0\0\0")]
    Para {
        channel: Channel,
        data: BusParaParameter,
    },

    #[brw(magic = b"iraP\x14\0\0\0")]
    Pari {
        channel: Channel,
        data: BusPariParameter,
    },

    #[brw(magic = b"mrpm\xf0\x01\0\0")]
    Fader {
        // #[brw(pad_size_to = 488, restore_position)]
        unknown: u32,

        #[brw(pad_before(480), restore_position)]
        length: u32,

        #[br(count = length)]
        records: Vec<Fader>,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum CompressorPayload {
    #[brw(magic = b"fdBM\xf4\x01\0\0")]
    MBdf {
        unknown_0: u32,
        unknown_1: u32,
        sample_rate_coefficients: [BiquadRate; 4],
        // #[br(count = size - (4 * std::mem::size_of::<BiquadRate>() as u32) - 8 - 8)]
        extra: [u32; 388 / 4],
    },
    #[brw(magic = b"txpc\x40\0\0\0")]
    Compressor {
        unknown: u32,
        filter_biquad: [f32; 5],
        attack_sec: f32,
        release_sec: f32,
        ratio: f32,
        knee_coeff: f32,
        threshold_db: f32,
        gain: f32,

        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "phantom-power value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "phantom-power value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        key_listen: bool,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum EqPayload {
    #[brw(magic = b"fdfL")]
    Lfdf {
        size: u32,
        unknown: u32,
        #[br(count = (size - 12)/4)]
        data: Vec<f32>,
    },

    #[brw(magic = b"fdBM\xf4\x01\0\0")]
    MBdf {
        unknown_0: u32,
        unknown_1: u32,
        sample_rate_coefficients: [BiquadRate; 4],
        // #[br(count = size - (4 * std::mem::size_of::<BiquadRate>() as u32) - 8 - 8)]
        extra: [u32; 388 / 4],
    },

    #[brw(magic = b"fdqB\x24\0\0\0")]
    Bqdf {
        unknown: u32,
        idx: u32,
        coefficients: [f32; 4],
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum GatePayload {
    #[brw(magic = b"fdBM\xf4\x01\0\0")]
    MBdf {
        unknown_0: u32,
        unknown_1: u32,
        sample_rate_coefficients: [BiquadRate; 4],
        // #[br(count = size - (4 * std::mem::size_of::<BiquadRate>() as u32) - 8 - 8)]
        extra: [u32; 388 / 4],
    },

    #[brw(magic = b"etag\x48\0\0\0")]
    Gate {
        unknown: u32,
        filter_biquad: [f32; 5],
        expander: f32,
        threshold: f32,
        attack_sec: f32,
        release_sec: f32,
        attack_coeff: f32,
        release_coeff: f32,
        unknown_val: u32,

        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "phantom-power value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "phantom-power value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        expander_auto: bool,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "phantom-power value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        key_listen: bool,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum LimiterPayload {
    #[brw(magic = b" mil\x18\0\0\0")]
    Lim {
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        smoothing_coeff: u32,
        threshold: f32,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, BinRead, BinWrite, PartialEq)]
pub enum VocalEffectPayload {
    #[brw(magic(b"xFoV\x10\0\0\0"))]
    VoFx {
        unknown: u32,
        option: VocalEffects,
    },

    #[brw(magic(b"ctob\x28\0\0\0"))]
    Filters {
        unknown: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        tune: u32,
        feedback: f32,
        damping: f32,
        dist: f32,
        unknown_6: f32,
        mix: f32,
    },

    #[brw(magic = b"vdog\x20\0\0\0")]
    Transformer {
        idx: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        lows: f32,
        width: f32,
        lows_2: f32,
        mix: f32,
    },

    #[brw(magic = b"hcev\x1c\0\0\0")]
    Delay {
        unknown_0: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        mix: f32,
        feedback: f32,
        time_sec: f32,
    },

    #[brw(magic = b"btob\x28\0\0\0")]
    RingModulator {
        unknown_0: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        freq: f32,
        dist: f32,
        unknown_4: f32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sub value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        sub: bool,
        sc_freq: f32,
        mix: f32,
    },

    #[brw(magic = b"atob\x28\0\0\0")]
    Vocoder {
        unknown_0: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        ty: VocoderType,
        freq: f32,
        unknown_4: f32,
        unknown_5: u32,
        unknown_6: u32,
        mix: f32,
    },

    #[brw(magic = b"4yam\x24\0\0\0")]
    DeTuner {
        unknown_0: u32,
        #[br(try_map = |value: u32| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enabled value must be zero or one",
            )),
        })]
        #[bw(map = |&enabled| if enabled { 1u32 } else { 0u32 })]
        enabled: bool,
        detune: u32,
        unknown_3: u32,
        unknown_4: u32,
        unknown_5: u32,
        mix: f32,
    },

    Unknown {
        tag: [u8; 4],
        size: u32,
        #[br(count = size.checked_sub(8).map_or(0, |x| x / 4))]
        data: Vec<u32>,
    },

    UnknownTag {
        tag: [u8; 4],
    },

    None,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
#[brw(repr = u32)]
pub enum VocoderType {
    Noise,
    Sawtooth,
    Rect,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum Store {
    #[brw(magic(b"MsrP"))]
    PresetMemory,

    #[brw(magic(b"tatS"))]
    Static,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub struct BiquadRate {
    coefficients: [f32; 5],
    sample_rate: f32,
}

fn f32tob(value: f32) -> Result<bool, Error> {
    match value {
        0.0 => Ok(false),
        1.0 => Ok(true),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "phantom-power value must be zero or one",
        )),
    }
}

fn u32tob(value: u32) -> Result<bool, Error> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "phantom-power value must be zero or one",
        )),
    }
}

fn btou32(value: &bool) -> u32 {
    if *value { 1u32 } else { 0u32 }
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum ApplParaParameter {
    #[brw(magic(1u32))]
    PhonesOut {
        value: f32,
    },

    #[brw(magic(2u32))]
    MainOut {
        value: f32,
    },

    #[brw(magic(3u32))]
    Gain {
        value: f32,
    },

    #[brw(magic(4u32))]
    Preset {
        #[br(try_map = f32tob)]
        #[bw(map = |&enabled| if enabled { 1.0_f32 } else { 0.0_f32 })]
        enabled: bool,
    },

    #[brw(magic(10u32))]
    Blend {
        value: f32,
    },

    #[brw(magic(14u32))]
    OutputDelay {
        sec: f32,
    },

    Unknown {
        id: u32,
        value: f32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum ApplPariParameter {
    #[brw(magic(0u32))]
    PhantomPower {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        enabled: bool,
    },

    #[brw(magic(9u32))]
    ChannelLink {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        enabled: bool,
    },

    #[brw(magic(11u32))]
    MonitorSource {
        source: Bus,
    },

    #[brw(magic(11u32))]
    VoiceEffects {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        disabled: bool,
    },

    #[brw(magic(13u32))]
    DelayChannel {
        channel: USBChannel,
    },

    #[brw(magic(16u32))]
    Preset {
        preset: u32,
    },

    #[brw(magic(17u32))]
    DualPreset {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        enabled: bool,
    },

    Unknown {
        id: u32,
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum DefaultParaParameter {
    #[brw(magic(1u32))]
    ReverbSend {
        value: f32,
    },

    #[brw(magic(2u32))]
    Pan {
        value: f32,
    },

    Unknown {
        id: u32,
        value: f32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum DefaultPariParameter {
    #[brw(magic(3u32))]
    USBFatChannel {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        enabled: bool,
    },

    Unknown {
        id: u32,
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum OptParaParameter {
    Unknown { id: u32, value: f32 },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum OptPariParameter {
    #[brw(magic(0u32))]
    ProcessingOrder {
        order: ProcessingOrder,
    },

    Unknown {
        id: u32,
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
#[brw(repr = u32)]
pub enum ProcessingOrder {
    CompFirst,
    EqFirst,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum BusParaParameter {
    #[brw(magic(0u32))]
    Playback {
        value: f32,
    },

    #[brw(magic(1u32))]
    VirtualA {
        value: f32,
    },

    #[brw(magic(2u32))]
    VirtualB {
        value: f32,
    },

    #[brw(magic(3u32))]
    Channel1 {
        value: f32,
    },

    #[brw(magic(4u32))]
    Channel2 {
        value: f32,
    },

    #[brw(magic(5u32))]
    Reverb {
        value: f32,
    },

    Unknown {
        id: u32,
        value: f32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum BusPariParameter {
    #[brw(magic(8u32))]
    MixMono {
        #[br(try_map = u32tob)]
        #[bw(map = btou32)]
        enabled: bool,
    },

    Unknown {
        id: u32,
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
#[brw(repr = u32)]
pub enum USBChannel {
    None = -1,
    USB12,
    USB34,
    USB56,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub struct JaSt {
    pub playback_left: f32,  // payload[0] dBFS
    pub playback_right: f32, // payload[1] dBFS

    pub virtual_a_left: f32,  // payload[2] dBFS
    pub virtual_a_right: f32, // payload[3] dBFS

    pub virtual_b_left: f32,  // payload[4] dBFS
    pub virtual_b_right: f32, // payload[5] dBFS

    pub stereo_left: f32,  // payload[6] dBFS
    pub channel_1: f32,    // payload[7] dBFS
    pub stereo_right: f32, // payload[8] dBFS
    pub channel_2: f32,    // payload[9] dBFS

    pub unknown_1_0: u32, // payload[10]
    pub unknown_1_1: u32, // payload[11]
    pub unknown_2_0: u32, // payload[12]
    pub unknown_2_1: u32, // payload[13]

    pub main_left: f32,   // payload[14] dBFS
    pub main_right: f32,  // payload[15] dBFS
    pub mix_a_left: f32,  // payload[16] dBFS
    pub mix_a_right: f32, // payload[17] dBFS
    pub mix_b_left: f32,  // payload[18] dBFS
    pub mix_b_right: f32, // payload[19] dBFS

    pub unknown_3_0: u32, // payload[20]
    pub unknown_3_1: u32, // payload[21]
    pub unknown_4_0: u32, // payload[22]
    pub unknown_4_1: u32, // payload[23]
    pub unknown_5_0: u32, // payload[24]
    pub unknown_5_1: u32, // payload[25]

    pub channel_1_gate: f32,    // payload[26] dBFS?
    pub channel_1_comp: f32,    // payload[27] dBFS
    pub channel_1_limiter: f32, // payload[28] dBFS

    pub channel_2_gate: f32,    // payload[29] dBFS?
    pub channel_2_comp: f32,    // payload[30] dBFS
    pub channel_2_limiter: f32, // payload[31] dBFS

    pub unknown_6_0: u32,        // payload[32]
    pub unknown_6_1: u32,        // payload[33]
    pub unknown_7_0: u32,        // payload[34]
    pub unknown_7_1: u32,        // payload[35]
    pub unknown_8_0: u32,        // payload[36]
    pub unknown_8_1: u32,        // payload[37]
    pub channel_1_activity: u32, // payload[38]
    pub channel_2_activity: u32, // payload[39]
    pub unknown_10_0: u32,       // payload[40]
    pub unknown_10_1: u32,       // payload[41]
    pub unknown_11_0: u32,       // payload[42]
    pub unknown_11_1: u32,       // payload[43]

    pub status_flags_0: StatusFlags0,

    pub status_flags_1: StatusFlags1, // payload[44]
    pub status_flags_2: u16,

    pub gain_headphones: f32, // payload[45] dBFS
    pub main_gain: f32,       // payload[46] dBFS
    pub monitor_mix: f32,     // payload[47] -1.0 to 1.0

    pub channel_1_gain: f32, // payload[48] 0 to 60
    pub channel_2_gain: f32, // payload[49] 0 to 60
    pub unknown_12_0: f32,   // payload[50]
    pub unknown_12_1: f32,   // payload[51]

    // unknown_13_0: u32, // payload[52]
    pub channel_1_flags: ChannelFlags,
    pub channel_2_flags: ChannelFlags,
    pub flag_2: u8, // affected
    pub flag_3: u8,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
#[brw(repr = u32)]
pub enum VocalEffects {
    Transformer,
    DeTuner,
    Vocoder,
    RingModulator,
    Filters,
    Delay,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub struct Fader {
    pub channel: MixChannel,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq, Enum, Eq)]
pub enum MixChannel {
    #[brw(magic(0u32))]
    Playback,
    #[brw(magic(1u32))]
    VirtualA,
    #[brw(magic(2u32))]
    VirtualB,
    #[brw(magic(3u32))]
    Channel1,
    #[brw(magic(4u32))]
    Channel2,
    #[brw(magic(5u32))]
    Reverb,

    // Unknown(u32),
}

impl From<u32> for MixChannel {
    fn from(value: u32) -> Self {
        match value {
            0 => MixChannel::Playback,
            1 => MixChannel::VirtualA,
            2 => MixChannel::VirtualB,
            3 => MixChannel::Channel1,
            4 => MixChannel::Channel2,
            5 => MixChannel::Reverb,
            _ => panic!("Invalid value for MixChannel: {}", value),
        }
    }
}

#[bitfield]
#[derive(Debug, Clone, Copy, PartialEq, BinRead, BinWrite)]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
pub struct StatusFlags0 {
    pub unknown_flag_0: bool,
    pub unknown_flag_1: bool,
    pub unknown_flag_2: bool,
    pub unknown_flag_3: bool,
    pub unknown_flag_4: bool,
    pub channel_1_preset_disabled: bool,
    pub channel_2_preset_disabled: bool,
    pub unknown_flag_7: bool,
}

#[bitfield]
#[derive(Debug, Clone, Copy, PartialEq, BinRead, BinWrite)]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
pub struct StatusFlags1 {
    pub unknown_flag_8: bool,
    pub unknown_flag_9: bool,
    pub unknown_flag_10: bool,
    pub unknown_flag_11: bool,
    pub channel_link: bool,
    pub unknown_flag_13: bool,
    pub unknown_flag_14: bool,
    pub unknown_flag_15: bool,
}

#[bitfield]
#[derive(Debug, Clone, Copy, PartialEq, BinRead, BinWrite)]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
pub struct ChannelFlags {
    pub enabled_48v: bool,
    pub unknown_flag_1: bool,
    pub unknown_flag_2: bool,
    pub unknown_flag_3: bool,
    pub unknown_flag_4: bool,
    pub unknown_flag_5: bool,
    pub unknown_flag_6: bool,
    pub unknown_flag_7: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trips_captured_pari_payload() {
        let bytes = [
            b'i', b'r', b'a', b'P', 20, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 1, 0, 0, 0,
        ];

        let payload = ApplPayload::read_le(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(
            payload,
            ApplPayload::Pari {
                channel: Channel::Channel1,
                data: ApplPariParameter::ChannelLink { enabled: true },
            }
        );

        let mut encoded = Cursor::new(Vec::new());
        payload.write_le(&mut encoded).unwrap();
        assert_eq!(encoded.into_inner(), bytes);
    }
}

#[derive(Debug, Clone, PartialEq, BinRead, BinWrite)]
#[br(import{ size: u32 })]
pub enum MemoryData {
    Preset {
        #[br(parse_with = parse_preset, args(size))]
        // #[bw(write_with = write_preset)]
        #[bw(try_map = serde_ub_json::to_bytes)]
        preset: Preset,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Preset {
    pub preset_name: String,
    // pub icon_id: String,
    // pub opt:
}

#[binrw::parser(reader, endian)]
fn parse_preset(size: u32) -> BinResult<Preset> {
    let mut buf = vec![0u8; size as usize];
    reader.read_exact(&mut buf)?; // or use binrw's read_to_end if you prefer

    // eprintln!("buf: {:X?}", buf);

    // from_vec(buf).map_err(|e| {
    from_bytes(&*buf).map_err(|e| {
        eprintln!("failed to parse: {e:#?}");
        binrw::Error::Custom {
            pos: 0,
            err: Box::new(Error::new(ErrorKind::InvalidData, e)),
        }
    })
}

#[binrw::writer(writer, endian)]
fn write_preset(this: &Preset) -> BinResult<()> {
    let bytes = to_vec(this).map_err(|e| {
        eprintln!("failed to write {e:?}");
        binrw::Error::Custom {
            pos: 0,
            err: Box::new(Error::new(ErrorKind::InvalidData, e)),
        }
    })?;

    writer.write_all(&bytes)?;
    Ok(())
}
