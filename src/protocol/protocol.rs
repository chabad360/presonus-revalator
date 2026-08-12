pub use super::payload::Payload;
use binrw::{BinRead, BinResult, BinWrite, Endian, binwrite};
use std::io::{Cursor, Seek, Write};

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Default)]
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

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Default)]
pub enum Target {
    #[default]
    #[brw(magic(b"lppA"))]
    Appl,
    #[brw(magic(b"comp"))]
    Comp,
    #[brw(magic(b"\0tpO"))]
    Opt,

    Unknown([u8; 4]),
}

#[derive(Debug, Clone, Copy, BinWrite, BinRead, Default)]
pub enum Direction {
    #[brw(magic(0x81u8))]
    IN,
    #[default]
    #[brw(magic(0x01u8))]
    OUT,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite, Default)]
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

#[derive(Debug, Clone, Copy, BinRead, BinWrite)]
pub struct Body {
    pub verb: Verb,
    pub object: Target,
    // selector: u32,
    #[brw(pad_before(4))]
    pub payload: Payload,
}
