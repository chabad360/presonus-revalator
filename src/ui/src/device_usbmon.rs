use crate::UsbEvent;
use crate::device_state::{ChannelState, DeviceState};
use nusb::{MaybeFuture, list_devices};
use pcap::Device;
use presonus_revalator::protocol::payload::{JaStBody, PayloadType};
use presonus_revalator::protocol::protocol::parse_message;
use std::io::{Error, ErrorKind};
use std::sync::mpsc;

pub fn init_capture() -> Result<(mpsc::Receiver<UsbEvent>, mpsc::Sender<DeviceState>), Error> {
    let (event_tx, event_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();

    let devices = Device::list().unwrap();
    let device = devices
        .iter()
        .find(|dev| dev.name == "usbmon3")
        .ok_or(Error::new(ErrorKind::NotFound, "usbmon3 not found"))?
        .clone();

    let _usb_device = list_devices()
        .wait()?
        .find(|dev| dev.vendor_id() == 0x194f && dev.product_id() == 0x0422)
        .ok_or(Error::new(ErrorKind::NotFound, "device not found"))?;

    let mut cap = device.open().unwrap();

    std::thread::spawn(move || {
        while let Ok(packet) = cap.next_packet() {
            let update = update_rx.try_recv();
            if update.is_ok() {
                println!("Received update: {:?}", update.unwrap());
            }

            if packet.data.len() <= 64 {
                continue;
            }

            let transfer_type = packet.data[9];
            let endpoint = packet.data[10];
            if transfer_type != 0x03 || endpoint != 0x81 {
                continue;
            }

            let msg = parse_message(&packet.data[64..]);
            if msg.is_err() || msg.unwrap().body.is_none() {
                continue;
            }

            let payload = msg.unwrap().body.unwrap().payload;
            if payload.payload_type != PayloadType::JaSt {
                continue;
            }
            if payload.payload.len() < size_of::<JaStBody>() {
                continue;
            }
            // let payload: JaStBody = unsafe { std::mem::transmute(payload) };
            let payload = unsafe { &*(payload.payload.as_ptr() as *const JaStBody) };

            let state = DeviceState {
                channels: vec![ChannelState {
                    name: "Channel 1".to_string(),
                    // value: payload.channel_1.log10() * 20f32,
                    value: payload.channel_1,
                    mute: false,
                    solo: false,
                    gain: payload.channel_1_gain,
                    gain_range: 0.0..=60.0,
                }],
            };

            event_tx.send(UsbEvent::State(state)).unwrap();
        }
    });

    Ok((event_rx, update_tx))
}
