use crate::device_state::{BoolParameters, Command, FloatParameters, ParameterValue, State, jast_parameter_values};
use binrw::BinRead;
use nusb::{MaybeFuture, list_devices};
use pcap::Device;
use presonus_revalator::protocol::payload::{ParaParameter, PariParameter, Payload};
use presonus_revalator::protocol::protocol::{Message, Target};
use std::io::{Cursor, Error, ErrorKind};
use std::sync::mpsc;

const USBMON_HEADER_LENGTH: usize = 64;
const BULK_TRANSFER: u8 = 0x03;
const OUT_ENDPOINT: u8 = 0x01;
const IN_ENDPOINT: u8 = 0x81;

pub fn init_capture() -> Result<(mpsc::Receiver<State>, mpsc::SyncSender<Command>), Error> {
    let (event_tx, event_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::sync_channel(2);

    let devices = Device::list().map_err(Error::other)?;
    let device = devices
        .into_iter()
        .find(|device| device.name == "usbmon3")
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "usbmon3 not found"))?;

    // Keep the same device-presence check as the real backend. usbmon is passive,
    // so opening the nusb device is neither necessary nor desirable here.
    let _usb_device = list_devices()
        .wait()?
        .find(|device| device.vendor_id() == 0x194f && device.product_id() == 0x0422)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "device not found"))?;

    let mut capture = device.open().map_err(Error::other)?;

    std::thread::spawn(move || {
        while let Ok(packet) = capture.next_packet() {
            // This backend cannot send commands to the device. Drain the channel so
            // UI controls remain usable while usbmon observes another application.
            for _command in update_rx.try_iter() {}

            let Some(message_data) = bulk_message_data(packet.data) else {
                continue;
            };

            let message = match Message::read_le(&mut Cursor::new(message_data)) {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("Error parsing usbmon message: {error:?}");
                    continue;
                }
            };

            let Some(state) = message_to_state(message) else {
                continue;
            };

            if event_tx.send(state).is_err() {
                break;
            }
        }
    });

    Ok((event_rx, update_tx))
}

fn bulk_message_data(packet: &[u8]) -> Option<&[u8]> {
    if packet.len() <= USBMON_HEADER_LENGTH || packet[9] != BULK_TRANSFER {
        return None;
    }

    // OUT payloads are captured on submission and IN payloads on completion.
    // The records for the opposite phase normally contain no data and are
    // naturally discarded by the length check above.
    if !matches!(packet[10], OUT_ENDPOINT | IN_ENDPOINT) {
        return None;
    }

    Some(&packet[USBMON_HEADER_LENGTH..])
}

fn message_to_state(message: Message) -> Option<State> {
    let body = message.body?;

    if !(matches!(body.object, Target::Appl(_) ) && (body.payload == Payload::AskJaSt || matches!(body.payload, Payload::JaSt { .. } ) || matches!(body.payload, Payload::MemP { .. }))) {
        println!("{body:?}")
    }

    match body.payload {
        Payload::JaSt(payload) => Some(State::UpdateMany(jast_parameter_values(payload))),
        Payload::Para {
            channel,
            data,
        } => {
            match data {
                ParaParameter::Gain { value } => Some(State::Update(ParameterValue::Float(
                    FloatParameters::Gain(channel as u8),
                    value,
                ))),
                ParaParameter::ProcessingOrder { raw_value } => {
                    // eprintln!("ProcesingOrder: {raw_value}");
                    None
                }
                ParaParameter::ReverbSend { value } => {
                    // eprintln!("ReverbSend: {value}");
                    None
                }
                ParaParameter::Pan { value } => {
                    // eprintln!("Pan: {value}");
                    None
                }
                ParaParameter::Preset { enabled } => Some(State::Update(ParameterValue::Bool(
                    BoolParameters::PresetEnabled(channel as u8),
                    enabled,
                ))),
                ParaParameter::Blend { .. } => None,
                ParaParameter::Unknown { id, value } => {
                    // eprintln!("Unknown Para: {id}: {value}");
                    None
                }
                ParaParameter::OutputDelay { .. } => None
            }
        }
        Payload::Pari {
            channel,
            data
        } => {
            match data {
                PariParameter::PhantomPower { enabled } => Some(State::Update(ParameterValue::Bool(
                    BoolParameters::PhantomPower(channel as u8),
                    enabled,
                ))),
                PariParameter::MixMono { enabled } => None,
                PariParameter::ChannelLink { enabled } => Some(State::Update(ParameterValue::Bool(
                    BoolParameters::ChannelLink(),
                    enabled,
                ))),
                PariParameter::MonitorSource { source } => {
                    // eprintln!("MonitorSource: {source:#?}");
                    None
                }
                PariParameter::Unknown { id, value } => {
                    // eprintln!("Unknown Pari: {id}: {value}");
                    None
                }
                PariParameter::USBFatChannel { .. } => None,
                PariParameter::DelayChannel { .. } => None,
                PariParameter::DualPreset { .. } => None,
                PariParameter::Preset { .. } => None,
            }
        }
        _ => None
        // Payload::AskJaSt | Payload::None => None,

        // other => {
        //     eprintln!("Unexpected payload in usbmon message: {other:#?}");
        //     None
        // }
        // //
        // // Payload::MemP { size, data }
        // // | Payload::MBdf { size, data }
        // // | Payload::Bqdf { size, data }
        // // | Payload::Lfdf { size, data }
        // // | Payload::MLff { size, data }
        // // | Payload::Gate { size, data }
        // // | Payload::Cpxt { size, data }
        // // | Payload::Lim { size, data } => {
        // //     let name = match body.payload {
        // //         Payload::MemP { .. } => "MemP",
        // //         Payload::MBdf { .. } => "MBdf",
        // //         Payload::Bqdf { .. } => "Bqdf",
        // //         Payload::Lfdf { .. } => "Lfdf",
        // //         Payload::MLff { .. } => "MLff",
        // //         Payload::Gate { .. } => "Gate",
        // //         Payload::Cpxt { .. } => "Cpxt",
        // //         Payload::Lim { .. } => "Lim",
        // //         _ => unreachable!(),
        // //     };
        // //     eprintln!("{name}: size={size}, data={data:?}");
        // //     None
        // // }
    }
}
