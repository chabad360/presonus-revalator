use derive_more::TryFrom;
use super::payload::{build_payload, parse_payload, Payload};

#[derive(Debug, Clone, Copy, TryFrom)]
#[try_from(repr)]
#[repr(u32)]
pub enum Verb {
    Rply = 0x52706c79,
    GetP = 0x47657450,
    SetP = 0x53657450,
}

#[derive(Debug, Clone, Copy, TryFrom)]
#[try_from(repr)]
#[repr(u32)]
pub enum Target {
    Appl = 0x4170706c,
    comp = 0x636f6d70,
}

#[derive(Debug, Clone, Copy, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub enum Direction {
    IN = 0x81,
    OUT = 0x01,
}

#[derive(Debug, Clone, Copy)]
pub struct Message<'a> {
    pub direction: Direction,
    pub sequence_number: u32,

    pub body: Option<Body<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Body<'a> {
    pub verb: Verb,
    pub object: Target,

    pub payload: Payload<'a>,
}

pub fn parse_message<T: AsRef<[u8]> + ?Sized>(buf: &T) -> Result<Message<'_>, ()> {
    let buf = buf.as_ref();
    if buf.len() < 8 {
        return Err(());
    }

    let total_length = u16::from_le_bytes([buf[0], buf[1]]);
    if buf.len() < total_length as usize {
        return Err(());
    }

    let direction = Direction::try_from(buf[3]).unwrap();
    let sequence_number = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if total_length == 8 {
        return Ok(Message {
            direction,
            sequence_number,
            body: None,
        });
    }

    let verb = Verb::try_from(u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]])).unwrap();
    let object =
        Target::try_from(u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]])).unwrap();
    let payload = parse_payload(&buf[20..total_length as usize]).unwrap();

    Ok(Message {
        direction,
        sequence_number,
        body: Some(Body {
            verb,
            object,
            payload,
        })
    })
}

pub fn build_message(msg: Message<'_>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(0x01);
    buf.push(msg.direction as u8);
    buf.extend_from_slice(&msg.sequence_number.to_le_bytes());
    if let Some(body) = msg.body {
        buf.extend_from_slice(&(body.verb as u32).to_le_bytes());
        buf.extend_from_slice(&(body.object as u32).to_le_bytes());
        buf.extend_from_slice(&[0x00; 4]); // padding
        buf.extend_from_slice(&build_payload(body.payload));
    };
    buf.insert(0, ((buf.len() as u16) + 2).to_le_bytes()[0]);
    buf.insert(1, ((buf.len() as u16) + 1).to_le_bytes()[1]);
    buf
}
