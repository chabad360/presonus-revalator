use binrw::{BinRead, BinWrite};

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub enum Payload {
    #[brw(
        magic = b"tSaJ\xdc\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\x80\x3f\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\xe1\x01"
    )]
    AskJaSt,

    #[brw(magic = b"tSaJ\xdc\0\0\0")]
    JaSt(JaSt),

    #[brw(magic = b"araP\x0c\0\0\0")]
    Para {
        channel: u32,
        parameter: u32,
        value: f32,
    },

    None,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq)]
pub struct JaSt {
    pub playback_left: f32,  // payload[0] dBFS
    pub playback_right: f32, // payload[1] dBFS

    pub virtual_a_left: f32,  // payload[2] dBFS
    pub virtual_a_right: f32, // payload[3] dBFS

    pub virtual_b_left: f32,  // payload[4] dBFS
    pub virtual_b_right: f32, // payload[5] dBFS

    pub channel_2_left: f32,  // payload[6] dBFS
    pub channel_1: f32,       // payload[7] dBFS
    pub channel_2_right: f32, // payload[8] dBFS
    pub channel_2: f32,       // payload[9] dBFS

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

    pub status_flags: u32, // payload[44]

    pub main_gain: f32,       // payload[45] dBFS
    pub gain_headphones: f32, // payload[46] dBFS
    pub monitor_mix: f32,     // payload[47] -1.0 to 1.0

    pub channel_1_gain: f32, // payload[48] 0 to 60
    pub channel_2_gain: f32, // payload[49] 0 to 60
    pub unknown_12_0: f32,   // payload[50]
    pub unknown_12_1: f32,   // payload[51]

    // unknown_13_0: u32, // payload[52]
    pub flag_0: u8, // payload[53] something to do with parameter changes, it might be a general event flag
    pub flag_1: u8,
    pub flag_2: u8, // affected
    pub flag_3: u8,
}
