use crate::device_state::{Command, Knob, Meter, ParameterValue, State};
use binrw::BinRead;
use nusb::{MaybeFuture, list_devices};
use pcap::Device;
use presonus_revalator::protocol::payload::Payload;
use presonus_revalator::protocol::protocol::Message;
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

    match body.payload {
        Payload::JaSt(payload) => Some(State::UpdateMany(vec![
            ParameterValue::Meter(
                Meter {
                    id: "meter_1",
                    range: 0.0..=1.0,
                },
                payload.channel_1,
            ),
            ParameterValue::Knob(
                Knob {
                    id: "gain_1",
                    range: 0.0..=60.0,
                },
                payload.channel_1_gain,
            ),
        ])),
        Payload::Para {
            channel: 0,
            parameter: 3,
            value,
        } => Some(State::Update(ParameterValue::Knob(
            Knob {
                id: "gain_1",
                range: 0.0..=60.0,
            },
            value,
        ))),
        _ => None,
    }
}
