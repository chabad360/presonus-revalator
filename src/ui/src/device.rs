use crate::device_state::{Command, Knob, Meter, ParameterValue, State, device_model};
use binrw::{BinRead, BinWrite};
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{Endpoint, MaybeFuture, list_devices};
use presonus_revalator::protocol::payload::Payload;
use presonus_revalator::protocol::protocol::{Body, Message, Target, Verb};
use std::io;
use std::io::{Cursor, Error, ErrorKind, Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

fn write_msg(writer: &mut impl Write, msg: &Message) -> Result<(), Error> {
    let out = msg.to_bytes().map_err(Error::other)?;
    writer.write_all(&out)?;
    writer.flush()
}

pub fn init_capture() -> Result<(mpsc::Receiver<State>, mpsc::SyncSender<Command>), Error> {
    let (event_tx, event_rx) = mpsc::channel();
    let (update_tx, mut update_rx) = mpsc::sync_channel(2);

    // let DEVICE = device_model();

    let device = list_devices()
        .wait()?
        .find(|dev| dev.vendor_id() == 0x194f && dev.product_id() == 0x0422)
        .ok_or(Error::new(ErrorKind::NotFound, "device not found"))?;

    let device = device.open().wait()?;
    let interface = device.detach_and_claim_interface(5).wait()?;
    println!(
        "Claimed interface: {:?}",
        interface.descriptors().collect::<Vec<_>>()
    );

    interface.set_alt_setting(1).wait()?;

    let out_endpoint = interface.endpoint::<Bulk, Out>(0x01)?;
    let in_endpoint = interface.endpoint::<Bulk, In>(0x81)?;

    let local_update = update_tx.clone();

    std::thread::spawn(move || {
        loop {
            local_update.send(Command::Refresh()).unwrap();
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    std::thread::spawn(move || {
        let mut writer = out_endpoint
            .writer(2048)
            .with_write_timeout(Duration::from_millis(1000))
            .with_num_transfers(1);
        let mut reader = in_endpoint
            .reader(2048)
            .with_read_timeout(Duration::from_millis(1000));
        let mut nonce = 0u8;
        let mut buf = [0u8; 8];

        loop {
            match process_command(update_rx.recv().unwrap(), nonce, &mut writer) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error processing command: {:?}", e);
                    writer = recover_writer(writer).unwrap();
                }
            }
            nonce = nonce.wrapping_add(1);
            match process_usb(event_tx.clone(), &mut reader, &mut buf) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error processing USB: {:?}", e);
                    reader = recover_reader(reader).unwrap()
                }
            }
        }
    });

    Ok((event_rx, update_tx))
}

fn process_command(
    cmd: Command,
    nonce: u8,
    mut writer: &mut EndpointWrite<Bulk>,
) -> Result<(), ProcessError> {
    match cmd {
        Command::Refresh() => Ok(write_msg(
            &mut writer,
            &Message {
                nonce,
                body: Some(Body {
                    verb: Verb::GetP,
                    object: Target::Appl,
                    payload: Payload::AskJaSt,
                }),
                ..Default::default()
            },
        )?),
        Command::Set(param) => Ok(write_msg(&mut writer, &param_to_msg(nonce, param))?),
        Command::SetMany(params) => {
            let params = params
                .iter()
                .map(|p| param_to_msg(nonce, p.clone()))
                .collect::<Vec<_>>();
            for param in params {
                write_msg(&mut writer, &param)?;
            }
            Ok(())
        }
    }
}

fn param_to_msg(nonce: u8, param: ParameterValue) -> Message {
    let mut payload = Payload::None;
    let mut msg = Message {
        nonce,
        ..Message::default()
    };
    match param {
        // ParameterValue::Button(button, value) => {
        //     m.body.as_mut().unwrap().verb = Verb::SetP;
        //     m.body.as_mut().unwrap().object = Target::Appl;
        //     m.body.as_mut().unwrap().payload.payload_type = PayloadType::JaSt;
        //     let mut payload = [0u8; 128];
        //     let payload_ptr = payload.as_mut_ptr() as *mut JaStBody;
        //     unsafe {
        //         (*payload_ptr).channel_1_gain = if value { 1.0 } else { 0.0 };
        //     }
        //     m.body.as_mut().unwrap().payload.payload = &payload;
        // },
        ParameterValue::Knob(knob, value) => {
            match knob.id {
                "gain_1" => {
                    payload = Payload::Para {
                        channel: 0,
                        parameter: 3,
                        value: value,
                    }
                }
                _ => {}
            }
            let body = Body {
                verb: Verb::SetP,
                object: Target::Appl,
                payload,
            };
            msg.body = Some(body);
        }
        _ => {}
    }
    msg
}

fn process_usb(
    event_tx: Sender<State>,
    reader: &mut EndpointRead<Bulk>,
    buf: &mut [u8; 8],
) -> Result<(), ProcessError> {
    reader.read_exact(buf)?;
    let total_length = u16::from_le_bytes([buf[0], buf[1]]);
    let mut message_buf = vec![0u8; total_length as usize];
    message_buf[..8].copy_from_slice(buf);

    if total_length > 8 {
        reader.read_exact(&mut message_buf[8..])?;
    }

    let msg = Message::read_le(&mut Cursor::new(message_buf))?;
    if let Some(body) = msg.body {
        match body.payload {
            Payload::JaSt(payload) => {
                event_tx.send(State::UpdateMany(vec![
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
                            range: 0.0..=1.0,
                        },
                        payload.channel_1_gain,
                    ),
                ]))?;
            }
            _ => {
                eprintln!("Received unexpected payload: {:?}", msg);
            }
        }
    }
    buf.fill(0);

    Ok(())
}

fn recover_reader(mut reader: EndpointRead<Bulk>) -> io::Result<EndpointRead<Bulk>> {
    reader.cancel_all();

    let mut endpoint = reader.into_inner();
    endpoint.cancel_all();
    while endpoint.pending() > 0 {
        endpoint.wait_next_complete(Duration::from_millis(100));
        endpoint.cancel_all();
    }
    // endpoint.clear_halt().wait()?;

    Ok(endpoint
        .reader(2048)
        .with_read_timeout(Duration::from_millis(1000))
        .with_num_transfers(1))
}

fn recover_writer(writer: EndpointWrite<Bulk>) -> io::Result<EndpointWrite<Bulk>> {
    let mut endpoint = writer.into_inner();
    endpoint.cancel_all();
    while endpoint.pending() > 0 {
        endpoint.wait_next_complete(Duration::from_millis(100));
        endpoint.cancel_all();
    }
    // endpoint.clear_halt().wait()?;

    Ok(endpoint
        .writer(2048)
        .with_write_timeout(Duration::from_millis(1000))
        .with_num_transfers(1))
}

#[derive(Debug)]
enum ProcessError {
    BinError(binrw::error::Error),
    SendError(mpsc::SendError<State>),
    RecvError(mpsc::TryRecvError),
    IoError(Error),
}

impl From<binrw::error::Error> for ProcessError {
    fn from(err: binrw::error::Error) -> Self {
        ProcessError::BinError(err)
    }
}

impl From<mpsc::SendError<State>> for ProcessError {
    fn from(err: mpsc::SendError<State>) -> Self {
        ProcessError::SendError(err)
    }
}

impl From<io::Error> for ProcessError {
    fn from(err: io::Error) -> Self {
        ProcessError::IoError(err)
    }
}

impl From<mpsc::TryRecvError> for ProcessError {
    fn from(err: mpsc::TryRecvError) -> Self {
        ProcessError::RecvError(err)
    }
}
