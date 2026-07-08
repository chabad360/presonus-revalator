use derive_more::TryFrom;

#[derive(Debug, Clone, Copy, TryFrom)]
#[try_from(repr)]
#[repr(u32)]
pub enum PayloadType {
    JaSt = 0x4a615374,
    Para = 0x50617261,
    MBdf = 0x4d426466,
    MemP = 0x4d656d50,
}

#[derive(Debug, Clone, Copy)]
pub struct Payload<'a> {
    pub payload_type: PayloadType,
    // pub payload: &'a [f32],
    pub payload: &'a [u8],
}

pub fn parse_payload<T: AsRef<[u8]> + ?Sized>(buf: &T) -> Result<Payload<'_>, &'static str> {
    let buf = buf.as_ref();
    if buf.len() < 8 {
        return Err("Buffer too short");
    }

    let payload_type =
        PayloadType::try_from(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])).unwrap();
    let total_length = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if buf.len() < total_length as usize {
        return Err("Buffer too short for total length");
    }
    let payload = &buf[8..total_length as usize];

    Ok(Payload {
        payload_type,
        payload,
        // payload: unsafe {
        //     std::slice::from_raw_parts(
        //         payload.as_ptr() as *const f32,
        //         payload.len() / std::mem::size_of::<f32>(),
        //     )
        // },
    })
}

pub fn build_payload(msg: Payload<'_>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(msg.payload_type as u32).to_le_bytes());
    buf.extend_from_slice(&((msg.payload.len() as u32) + 8).to_le_bytes());
    buf.extend_from_slice(msg.payload);
    // for &word in msg.payload {
    //     buf.extend_from_slice(&word.to_le_bytes());
    // }
    buf
}
