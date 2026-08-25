pub use super::payload::Payload;
use binrw::{BinRead, BinResult, BinWrite};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Default, Eq, PartialEq)]
pub enum Verb {
    #[brw(magic(b"ylpR"))]
    Rply,
    #[default]
    #[brw(magic(b"PteG"))]
    GetP,
    #[brw(magic(b"PteS"))]
    SetP,

    Unknown([u8; 4]),
}

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Eq, PartialEq)]
pub enum Target {
    #[brw(magic(b"lppA"))]
    Appl(Selector),

    #[brw(magic(0u32))]
    None(Selector),
    #[brw(magic(b"pmoc"))]
    Comp(Channel),
    #[brw(magic(b" tpo"))]
    Opt(Channel),
    #[brw(magic(b"  qe"))]
    Eq(Channel),
    #[brw(magic(b" mil"))]
    Lim(Channel),
    #[brw(magic(b"etag"))]
    Gate(Channel),
    #[brw(magic(b"tlif"))]
    Filt(Channel),

    #[brw(magic(0x000000CAu32))]
    _CA(u32),
    #[brw(magic(0x000000C9u32))]
    VocalEffect(Channel),
    #[brw(magic(b"\0\0\0@"))]
    At(u32),
    #[brw(magic(100u32))]
    Bus(Bus),

    Unknown(u32, u32),
}

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Default)]
pub enum Direction {
    #[brw(magic(0x81u8))]
    IN,
    #[default]
    #[brw(magic(0x01u8))]
    OUT,
}

#[derive(Debug, Clone, BinRead, BinWrite, Default)]
#[brw(little)]
pub struct Message {
    pub length: u16,
    #[brw(magic(0x01u8))]
    pub direction: Direction,

    pub nonce: u8,
    pub status: u8,

    #[br(if(length > 8))]
    #[brw(pad_before(2))]
    pub body: Option<Body>,
}

impl Message {
    pub fn to_bytes(&self) -> BinResult<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());

        self.write(&mut output)?;

        let mut bytes = output.into_inner();
        let length = u16::try_from(bytes.len()).expect("protocol message exceeds u16::MAX bytes");

        bytes[..2].copy_from_slice(&length.to_le_bytes());

        Ok(bytes)
    }
}

#[derive(Debug, Clone, BinRead, BinWrite)]
pub struct Body {
    pub verb: Verb,
    pub object: Target,
    // pub Bus: Bus,
    // #[brw(pad_before(4))]
    pub payload: Payload,
}


#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq, Eq, Default)]
#[brw(repr = u32)]
pub enum Selector {
    #[default]
    None
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq, Eq)]
#[brw(repr = u32)]
pub enum Bus {
    MainMix,
    MixA,
    MixB,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, PartialEq, Eq)]
#[brw(repr = u32)]
pub enum Channel {
    Channel1,
    Channel2,
}

impl From<u8> for Channel {
    fn from(value: u8) -> Self {
        match value {
            0 => Channel::Channel1,
            1 => Channel::Channel2,
            _ => panic!("Invalid channel value: {}", value),
        }
    }
}