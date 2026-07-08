use std::io::{Error, ErrorKind};
use nusb::{MaybeFuture, list_devices};
use pcap::Device;

use presonus_revalator::protocol::protocol::parse_message;

fn main() {
    let devices = Device::list().unwrap();
    let device = devices.iter().find(|dev| dev.name == "usbmon3").unwrap().clone();

    let usb_device = list_devices()
        .wait().unwrap()
        .find(|dev| dev.vendor_id() == 0x194f && dev.product_id() == 0x0422)
        .ok_or(Error::new(ErrorKind::NotFound, "device not found")).unwrap();

    let mut cap = device.open().unwrap();

    while let Ok(packet) = cap.next_packet() {
        if packet.data.len() <= 64 {
            continue;
        }

        let transfer_type = packet.data[09];
        let endpoint = packet.data[10];
        if transfer_type != 0x03 || endpoint != 0x81 {
            continue;
        }

        println!("{:?}", &parse_message(&packet.data[64..]));
    }

}