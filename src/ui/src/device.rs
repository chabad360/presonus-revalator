use crate::device_state::{
    BoolParameters, Command, FloatParameters, ParameterValue, State, jast_parameter_values,
};
use anyhow::{Result, bail};
use binrw::BinRead;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{MaybeFuture, list_devices};
use presonus_revalator::protocol::payload::{ParaParameter, PariParameter, Payload};
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
            let messages = match next_messages(nonce, &mut update_rx) {
                Ok(messages) => messages,
                Err(error) => {
                    eprintln!("Error processing command: {error:?}");
                    continue;
                }
            };

            let mut messages = messages.into_iter().peekable();
            while let Some(message) = messages.next() {
                if let Err(error) = write_msg(&mut writer, &message) {
                    eprintln!("Error writing USB message: {error:?}");
                    writer = recover_writer(writer).unwrap();
                    break;
                }

                nonce = nonce.wrapping_add(1);

                if let Err(error) = process_usb(event_tx.clone(), &mut reader, &mut buf) {
                    eprintln!("Error processing USB: {error:?}");
                    reader = recover_reader(reader).unwrap();
                    break;
                }

                // if messages.peek().is_some() && is_channel_link_enable(&message) {
                //     // The captured controller waits about 23 ms after enabling
                //     // target 0 before sending the target-1 link update.
                //     std::thread::sleep(Duration::from_millis(25));
                // }
            }
        }
    });

    Ok((event_rx, update_tx))
}

// fn is_channel_link_enable(message: &Message) -> bool {
//     matches!(
//         message.body.as_ref().map(|body| body.payload),
//         Some(Payload::Pari {
//             data: PariParameter::ChannelLink { enabled: true },
//             ..
//         })
//     )
// }

fn next_messages(nonce: u8, update_rx: &mut Receiver<Command>) -> anyhow::Result<Vec<Message>> {
    match update_rx.recv()? {
        Command::Refresh() => Ok(vec![Message {
            nonce,
            body: Some(Body {
                verb: Verb::GetP,
                object: Target::Appl,
                payload: Payload::AskJaSt,
            }),
            ..Default::default()
        }]),
        Command::Set(param) => param_to_msgs(nonce, param),
        Command::SetMany(params) => {
            let mut messages = Vec::new();
            for parameter in params {
                let message_nonce = nonce.wrapping_add(messages.len() as u8);
                messages.extend(param_to_msgs(message_nonce, parameter)?);
            }
            Ok(messages)
        }
    }
}

fn param_to_msgs(nonce: u8, param: ParameterValue) -> Result<Vec<Message>> {
    if let ParameterValue::Bool(BoolParameters::ChannelLink(), enabled) = param {
        return Ok((0..=1)
            .map(|channel| {
                message(
                    nonce.wrapping_add(channel as u8),
                    Payload::Pari {
                        channel,
                        data: PariParameter::ChannelLink { enabled },
                    },
                )
            })
            .collect());
    }

    if let ParameterValue::Bool(BoolParameters::LinkedPresetEnabled(), enabled) = param {
        return Ok((0..=1)
            .map(|channel| {
                message(
                    nonce.wrapping_add(channel as u8),
                    Payload::Para {
                        channel,
                        data: ParaParameter::Preset { enabled },
                    },
                )
            })
            .collect());
    }

    Ok(vec![param_to_msg(nonce, param)?])
}

fn param_to_msg(nonce: u8, param: ParameterValue) -> Result<Message> {
    let payload = match param {
        ParameterValue::Float(FloatParameters::Gain(channel), value) => Payload::Para {
            channel: channel.into(),
            data: ParaParameter::Gain { value },
        },
        ParameterValue::Bool(BoolParameters::PresetEnabled(channel), enabled) => Payload::Para {
            channel: channel.into(),
            data: ParaParameter::Preset { enabled },
        },
        ParameterValue::Bool(BoolParameters::PhantomPower(channel), enabled) => Payload::Pari {
            channel: channel.into(),
            data: PariParameter::PhantomPower { enabled },
        },
        unsupported => bail!("parameter is not writable: {unsupported:?}"),
    };

    Ok(message(nonce, payload))
}

fn message(nonce: u8, payload: Payload) -> Message {
    Message {
        nonce,
        body: Some(Body {
            verb: Verb::SetP,
            object: Target::Appl,
            payload,
        }),
        ..Message::default()
    }
}

fn process_usb(
    event_tx: Sender<State>,
    reader: &mut EndpointRead<Bulk>,
    buf: &mut [u8; 8],
) -> anyhow::Result<()> {
    reader.read_exact(buf)?;
    let total_length = u16::from_le_bytes([buf[0], buf[1]]);
    let mut message_buf = vec![0u8; total_length as usize];
    message_buf[..8].copy_from_slice(buf);

    if total_length > 8 {
        reader.read_exact(&mut message_buf[8..])?;
    }

    let msg = Message::read_le(&mut Cursor::new(message_buf))?;
    if let Some(ref body) = msg.body {
        match body.payload {
            Payload::JaSt(payload) => {
                event_tx.send(State::UpdateMany(jast_parameter_values(payload)))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_phantom_power_as_pari() {
        for channel in 0..=1 {
            let enabled = param_to_msg(
                7,
                ParameterValue::Bool(BoolParameters::PhantomPower(channel), true),
            )
                .unwrap()
                .to_bytes()
                .unwrap();
            let disabled = param_to_msg(
                7,
                ParameterValue::Bool(BoolParameters::PhantomPower(channel), false),
            )
                .unwrap()
                .to_bytes()
                .unwrap();

            assert_pari(&enabled, channel.into(), 0, 1);
            assert_pari(&disabled, channel.into(), 0, 0);
        }
    }

    #[test]
    fn serializes_channel_link_for_both_targets() {
        let messages =
            param_to_msgs(7, ParameterValue::Bool(BoolParameters::ChannelLink(), true)).unwrap();
        let bytes = messages
            .iter()
            .map(Message::to_bytes)
            .collect::<binrw::BinResult<Vec<_>>>()
            .unwrap();

        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0][4], 7);
        assert_eq!(bytes[1][4], 8);
        assert_pari(&bytes[0], 0, 9, 1);
        assert_pari(&bytes[1], 1, 9, 1);
    }

    #[test]
    fn serializes_linked_preset_for_both_channels() {
        let messages = param_to_msgs(
            11,
            ParameterValue::Bool(BoolParameters::LinkedPresetEnabled(), true),
        )
            .unwrap();
        let bytes = messages
            .iter()
            .map(Message::to_bytes)
            .collect::<binrw::BinResult<Vec<_>>>()
            .unwrap();

        assert_eq!(bytes.len(), 2);
        for (channel, bytes) in bytes.iter().enumerate() {
            assert_eq!(bytes[4], 11 + channel as u8);
            assert_eq!(&bytes[20..24], b"araP");
            assert_eq!(&bytes[28..32], &(channel as u32).to_le_bytes());
            assert_eq!(&bytes[32..36], &4_u32.to_le_bytes());
            assert_eq!(&bytes[36..40], &1.0_f32.to_le_bytes());
        }
    }

    fn assert_pari(bytes: &[u8], target_index: u32, parameter_id: u32, value: u32) {
        assert_eq!(&bytes[..2], &40_u16.to_le_bytes());
        assert_eq!(&bytes[8..12], b"PteS");
        assert_eq!(&bytes[12..16], b"lppA");
        assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
        assert_eq!(&bytes[20..24], b"iraP");
        assert_eq!(&bytes[24..28], &20_u32.to_le_bytes());
        assert_eq!(&bytes[28..32], &target_index.to_le_bytes());
        assert_eq!(&bytes[32..36], &parameter_id.to_le_bytes());
        assert_eq!(&bytes[36..40], &value.to_le_bytes());
        assert_eq!(bytes.len(), 40);
    }

    #[test]
    fn serializes_preset_enable_and_disable_as_protocol_booleans() {
        let enabled = param_to_msg(
            7,
            ParameterValue::Bool(BoolParameters::PresetEnabled(1), true),
        )
            .unwrap()
            .to_bytes()
            .unwrap();
        let disabled = param_to_msg(
            7,
            ParameterValue::Bool(BoolParameters::PresetEnabled(1), false),
        )
            .unwrap()
            .to_bytes()
            .unwrap();

        assert_eq!(enabled.len(), disabled.len());
        assert_eq!(
            &enabled[..enabled.len() - 4],
            &disabled[..disabled.len() - 4]
        );
        assert_eq!(
            &enabled[enabled.len() - 12..enabled.len() - 8],
            &1_u32.to_le_bytes()
        );
        assert_eq!(
            &enabled[enabled.len() - 8..enabled.len() - 4],
            &4_u32.to_le_bytes()
        );
        assert_eq!(&enabled[enabled.len() - 4..], &1.0_f32.to_le_bytes());
        assert_eq!(&disabled[disabled.len() - 4..], &0.0_f32.to_le_bytes());
    }
}
