use crate::device_state::{
    BoolParameters, ChoiceParameters, Command, FloatParameters, ParameterValue, State,
    jast_parameter_values,
};
use anyhow::{Result, bail};
use binrw::BinRead;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{MaybeFuture, list_devices};
use presonus_revalator::protocol::payload::{
    ApplParaParameter, ApplPariParameter, ApplPayload, BusParaParameter, BusPariParameter,
    BusPayload, DefaultParaParameter, DefaultPayload, Fader, MixChannel, OptPariParameter,
    OptPayload, ProcessingOrder,
};
use presonus_revalator::protocol::protocol::{Body, Bus, Channel, Message, Selector, Target, Verb};
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
        local_update.send(Command::Initialize()).unwrap();
        let mut refresh_count = 0;
        loop {
            local_update.send(Command::Refresh()).unwrap();
            if refresh_count % 20 == 0 {
                local_update.send(Command::RefreshMixers()).unwrap();
            }
            refresh_count += 1;
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

            for message in messages {
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
            }
        }
    });

    Ok((event_rx, update_tx))
}

fn next_messages(nonce: u8, update_rx: &mut Receiver<Command>) -> anyhow::Result<Vec<Message>> {
    match update_rx.recv()? {
        Command::Initialize() => Ok(startup_refresh_messages(nonce)),
        Command::Refresh() => Ok(vec![Message {
            nonce,
            body: Some(Body {
                verb: Verb::GetP,
                object: Target::Appl(Selector::None, ApplPayload::AskJaSt),
            }),
            ..Default::default()
        }]),
        Command::RefreshMixers() => Ok(mixer_refresh_messages(nonce)),
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

fn startup_refresh_messages(nonce: u8) -> Vec<Message> {
    let mut messages = Vec::with_capacity(31);
    for channel in 0..=1 {
        messages.push(get_message(
            nonce.wrapping_add(messages.len() as u8),
            Target::Default(
                Selector::None,
                DefaultPayload::Para {
                    channel: channel.into(),
                    data: DefaultParaParameter::Pan { value: 0.5 },
                },
            ),
        ));
        messages.push(get_message(
            nonce.wrapping_add(messages.len() as u8),
            Target::Default(
                Selector::None,
                DefaultPayload::Para {
                    channel: channel.into(),
                    data: DefaultParaParameter::ReverbSend { value: 0.0 },
                },
            ),
        ));
        messages.push(get_message(
            nonce.wrapping_add(messages.len() as u8),
            Target::Opt(
                channel.into(),
                OptPayload::Pari {
                    channel: Channel::Channel1,
                    data: OptPariParameter::ProcessingOrder {
                        order: ProcessingOrder::CompFirst,
                    },
                },
            ),
        ));
    }
    for data in [
        ApplParaParameter::PhonesOut { value: -96.0 },
        ApplParaParameter::MainOut { value: -96.0 },
        ApplParaParameter::Blend { value: 0.0 },
    ] {
        messages.push(get_message(
            nonce.wrapping_add(messages.len() as u8),
            Target::Appl(
                Selector::None,
                ApplPayload::Para {
                    channel: Channel::Channel1,
                    data,
                },
            ),
        ));
    }
    messages.push(get_message(
        nonce.wrapping_add(messages.len() as u8),
        Target::Appl(
            Selector::None,
            ApplPayload::Pari {
                channel: Channel::Channel1,
                data: ApplPariParameter::MonitorSource {
                    source: Bus::MainMix,
                },
            },
        ),
    ));

    let mixer_nonce = nonce.wrapping_add(messages.len() as u8);
    messages.extend(mixer_refresh_messages(mixer_nonce));
    messages
}

fn mixer_refresh_messages(nonce: u8) -> Vec<Message> {
    let mut messages = Vec::with_capacity(21);
    for bus in 0..=2 {
        let bus = mix_bus(bus).expect("known mixer bus");
        for source in 0..=5 {
            messages.push(get_message(
                nonce.wrapping_add(messages.len() as u8),
                Target::Bus(
                    bus,
                    BusPayload::Para {
                        channel: Channel::Channel1,
                        data: mix_fader(source, f32::NAN).expect("known mixer source"),
                    },
                    // BusPayload::Fader {
                    //     unknown: 0,
                    //     length: 5,
                    //     records: vec![
                    //         Fader {
                    //             channel: MixChannel::Playback,
                    //             value: 0.0,
                    //         },
                    //         Fader {
                    //             channel: MixChannel::VirtualA,
                    //             value: 0.0,
                    //         },
                    //         Fader {
                    //             channel: MixChannel::VirtualB,
                    //             value: 0.0,
                    //         },
                    //         Fader {
                    //             channel: MixChannel::Channel1,
                    //             value: 0.0,
                    //         },
                    //         Fader {
                    //             channel: MixChannel::Channel2,
                    //             value: 0.0
                    //         },
                    //         Fader {
                    //             channel: MixChannel::Reverb,
                    //             value: 0.0
                    //         }
                    //     ],
                    // },
                ),
            ));
        }
        messages.push(get_message(
            nonce.wrapping_add(messages.len() as u8),
            Target::Bus(
                bus,
                BusPayload::Pari {
                    channel: Channel::Channel1,
                    data: BusPariParameter::MixMono { enabled: false },
                },
            ),
        ));
    }
    messages
}

fn param_to_msgs(nonce: u8, param: ParameterValue) -> Result<Vec<Message>> {
    if let ParameterValue::Bool(BoolParameters::ChannelLink(), enabled) = param {
        return Ok((0..=1)
            .map(|channel| {
                appl_message(
                    nonce.wrapping_add(channel),
                    ApplPayload::Pari {
                        channel: channel.into(),
                        data: ApplPariParameter::ChannelLink { enabled },
                    },
                )
            })
            .collect());
    }

    if let ParameterValue::Bool(BoolParameters::LinkedPresetEnabled(), enabled) = param {
        return Ok((0..=1)
            .map(|channel| {
                appl_message(
                    nonce.wrapping_add(channel),
                    ApplPayload::Para {
                        channel: channel.into(),
                        data: ApplParaParameter::Preset { enabled },
                    },
                )
            })
            .collect());
    }

    Ok(vec![param_to_msg(nonce, param)?])
}

fn param_to_msg(nonce: u8, param: ParameterValue) -> Result<Message> {
    let object = match param {
        ParameterValue::Float(FloatParameters::Gain(channel), value) => Target::Appl(
            Selector::None,
            ApplPayload::Para {
                channel: channel.into(),
                data: ApplParaParameter::Gain { value },
            },
        ),
        ParameterValue::Float(FloatParameters::HeadphonesGain(), value) => Target::Appl(
            Selector::None,
            ApplPayload::Para {
                channel: Channel::Channel1,
                data: ApplParaParameter::PhonesOut { value },
            },
        ),
        ParameterValue::Float(FloatParameters::MainGain(), value) => Target::Appl(
            Selector::None,
            ApplPayload::Para {
                channel: Channel::Channel1,
                data: ApplParaParameter::MainOut { value },
            },
        ),
        ParameterValue::Float(FloatParameters::MonitorMix(), value) => {
            if !(-1.0..=1.0).contains(&value) {
                bail!("blend must be between -1.0 and 1.0: {value}");
            }
            Target::Appl(
                Selector::None,
                ApplPayload::Para {
                    channel: Channel::Channel1,
                    data: ApplParaParameter::Blend { value },
                },
            )
        }
        ParameterValue::Float(FloatParameters::Pan(channel), value) => {
            if !(0.0..=1.0).contains(&value) {
                bail!("pan must be between 0.0 and 1.0: {value}");
            }
            Target::Default(
                Selector::None,
                DefaultPayload::Para {
                    channel: channel.into(),
                    data: DefaultParaParameter::Pan { value },
                },
            )
        }
        ParameterValue::Float(FloatParameters::ReverbSend(channel), value) => Target::Default(
            Selector::None,
            DefaultPayload::Para {
                channel: channel.into(),
                data: DefaultParaParameter::ReverbSend { value },
            },
        ),
        ParameterValue::Float(FloatParameters::MixFader(bus, source), value) => Target::Bus(
            mix_bus(bus)?,
            BusPayload::Fader {
                unknown: 0,
                length: 1,
                records: vec![Fader {
                    channel: source,
                    value,
                }],
            },
        ),
        ParameterValue::Bool(BoolParameters::PresetEnabled(channel), enabled) => Target::Appl(
            Selector::None,
            ApplPayload::Para {
                channel: channel.into(),
                data: ApplParaParameter::Preset { enabled },
            },
        ),
        ParameterValue::Bool(BoolParameters::PhantomPower(channel), enabled) => Target::Appl(
            Selector::None,
            ApplPayload::Pari {
                channel: channel.into(),
                data: ApplPariParameter::PhantomPower { enabled },
            },
        ),
        ParameterValue::Bool(BoolParameters::MixMono(bus), enabled) => Target::Bus(
            mix_bus(bus)?,
            BusPayload::Pari {
                channel: Channel::Channel1,
                data: BusPariParameter::MixMono { enabled },
            },
        ),
        ParameterValue::Choice(ChoiceParameters::MonitorSource(), source) => Target::Appl(
            Selector::None,
            ApplPayload::Pari {
                channel: Channel::Channel1,
                data: ApplPariParameter::MonitorSource {
                    source: monitor_source(source)?,
                },
            },
        ),
        ParameterValue::Choice(ChoiceParameters::ProcessingOrder(channel), order) => Target::Opt(
            channel.into(),
            OptPayload::Pari {
                channel: Channel::Channel1,
                data: OptPariParameter::ProcessingOrder {
                    order: processing_order(order)?,
                },
            },
        ),
        unsupported => bail!("parameter is not writable: {unsupported:?}"),
    };

    Ok(message(nonce, object))
}

fn processing_order(value: u32) -> Result<ProcessingOrder> {
    match value {
        0 => Ok(ProcessingOrder::CompFirst),
        1 => Ok(ProcessingOrder::EqFirst),
        _ => bail!("invalid processing order: {value}"),
    }
}

fn monitor_source(value: u32) -> Result<Bus> {
    match value {
        0 => Ok(Bus::MainMix),
        1 => Ok(Bus::MixA),
        2 => Ok(Bus::MixB),
        _ => bail!("invalid monitor source: {value}"),
    }
}

fn mix_bus(value: u8) -> Result<Bus> {
    match value {
        0 => Ok(Bus::MainMix),
        1 => Ok(Bus::MixA),
        2 => Ok(Bus::MixB),
        _ => bail!("invalid mixer bus: {value}"),
    }
}

fn get_message(nonce: u8, object: Target) -> Message {
    Message {
        nonce,
        body: Some(Body {
            verb: Verb::GetP,
            object,
        }),
        ..Message::default()
    }
}

fn appl_message(nonce: u8, payload: ApplPayload) -> Message {
    message(nonce, Target::Appl(Selector::None, payload))
}

fn mix_fader(source: u8, value: f32) -> Result<BusParaParameter> {
    match source {
        0 => Ok(BusParaParameter::Playback { value }),
        1 => Ok(BusParaParameter::VirtualA { value }),
        2 => Ok(BusParaParameter::VirtualB { value }),
        3 => Ok(BusParaParameter::Channel1 { value }),
        4 => Ok(BusParaParameter::Channel2 { value }),
        5 => Ok(BusParaParameter::Reverb { value }),
        _ => bail!("invalid mixer source: {source}"),
    }
}

fn mix_fader_value(parameter: BusParaParameter) -> Option<(MixChannel, f32)> {
    match parameter {
        BusParaParameter::Playback { value } => Some((MixChannel::Playback, value)),
        BusParaParameter::VirtualA { value } => Some((MixChannel::VirtualA, value)),
        BusParaParameter::VirtualB { value } => Some((MixChannel::VirtualB, value)),
        BusParaParameter::Channel1 { value } => Some((MixChannel::Channel1, value)),
        BusParaParameter::Channel2 { value } => Some((MixChannel::Channel2, value)),
        BusParaParameter::Reverb { value } => Some((MixChannel::Reverb, value)),
        BusParaParameter::Unknown { .. } => None,
    }
}

fn message(nonce: u8, object: Target) -> Message {
    Message {
        nonce,
        body: Some(Body {
            verb: Verb::SetP,
            object,
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
        match body.object {
            Target::Appl(_, ApplPayload::JaSt(payload)) => {
                event_tx.send(State::UpdateMany(jast_parameter_values(payload)))?;
            }
            Target::Appl(_, ApplPayload::Para { channel, data }) => {
                let parameter = match data {
                    ApplParaParameter::PhonesOut { value } => Some(ParameterValue::Float(
                        FloatParameters::HeadphonesGain(),
                        value,
                    )),
                    ApplParaParameter::MainOut { value } => {
                        Some(ParameterValue::Float(FloatParameters::MainGain(), value))
                    }
                    ApplParaParameter::Gain { value } => Some(ParameterValue::Float(
                        FloatParameters::Gain(channel as u8),
                        value,
                    )),
                    ApplParaParameter::Preset { enabled } => Some(ParameterValue::Bool(
                        BoolParameters::PresetEnabled(channel as u8),
                        enabled,
                    )),
                    ApplParaParameter::Blend { value } => {
                        Some(ParameterValue::Float(FloatParameters::MonitorMix(), value))
                    }
                    ApplParaParameter::OutputDelay { .. } | ApplParaParameter::Unknown { .. } => {
                        None
                    }
                };
                if let Some(parameter) = parameter {
                    event_tx.send(State::Update(parameter))?;
                }
            }
            Target::Appl(
                _,
                ApplPayload::Pari {
                    channel: Channel::Channel1,
                    data: ApplPariParameter::MonitorSource { source },
                },
            ) => {
                event_tx.send(State::Update(ParameterValue::Choice(
                    ChoiceParameters::MonitorSource(),
                    source as u32,
                )))?;
            }
            Target::Default(_, DefaultPayload::Para { channel, data }) => {
                let parameter = match data {
                    DefaultParaParameter::Pan { value } => Some(ParameterValue::Float(
                        FloatParameters::Pan(channel as u8),
                        value,
                    )),
                    DefaultParaParameter::ReverbSend { value } => Some(ParameterValue::Float(
                        FloatParameters::ReverbSend(channel as u8),
                        value,
                    )),
                    DefaultParaParameter::Unknown { .. } => None,
                };
                if let Some(parameter) = parameter {
                    event_tx.send(State::Update(parameter))?;
                }
            }
            Target::Opt(
                channel,
                OptPayload::Pari {
                    channel: Channel::Channel1,
                    data: OptPariParameter::ProcessingOrder { order },
                },
            ) => {
                event_tx.send(State::Update(ParameterValue::Choice(
                    ChoiceParameters::ProcessingOrder(channel as u8),
                    order as u32,
                )))?;
            }
            Target::Bus(bus, BusPayload::Para { data, .. }) => {
                if let Some((source, value)) = mix_fader_value(data) {
                    event_tx.send(State::Update(ParameterValue::Float(
                        FloatParameters::MixFader(bus as u8, source),
                        value,
                    )))?;
                }
            }
            Target::Bus(
                bus,
                BusPayload::Pari {
                    data: BusPariParameter::MixMono { enabled },
                    ..
                },
            ) => {
                event_tx.send(State::Update(ParameterValue::Bool(
                    BoolParameters::MixMono(bus as u8),
                    enabled,
                )))?;
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

    #[test]
    fn serializes_processing_order() {
        for (channel, value, order) in [
            (0, 0, ProcessingOrder::CompFirst),
            (1, 1, ProcessingOrder::EqFirst),
        ] {
            let message = param_to_msg(
                7,
                ParameterValue::Choice(ChoiceParameters::ProcessingOrder(channel), value),
            )
            .unwrap();
            let bytes = message.to_bytes().unwrap();

            assert_eq!(&bytes[12..16], b" tpo");
            assert_eq!(&bytes[16..20], &(channel as u32).to_le_bytes());
            assert_eq!(&bytes[20..24], b"iraP");
            assert_eq!(&bytes[28..32], &0_u32.to_le_bytes());
            assert_eq!(&bytes[32..36], &0_u32.to_le_bytes());
            assert_eq!(&bytes[36..40], &value.to_le_bytes());
            assert_eq!(
                message.body.unwrap().object,
                Target::Opt(
                    channel.into(),
                    OptPayload::Pari {
                        channel: Channel::Channel1,
                        data: OptPariParameter::ProcessingOrder { order },
                    },
                )
            );
        }

        assert!(
            param_to_msg(
                7,
                ParameterValue::Choice(ChoiceParameters::ProcessingOrder(0), 2),
            )
            .is_err()
        );
    }

    #[test]
    fn serializes_monitor_source() {
        for (value, source) in [(0, Bus::MainMix), (1, Bus::MixA), (2, Bus::MixB)] {
            let message = param_to_msg(
                7,
                ParameterValue::Choice(ChoiceParameters::MonitorSource(), value),
            )
            .unwrap();
            let bytes = message.to_bytes().unwrap();

            assert_pari(&bytes, 0, 11, value);
            assert_eq!(
                message.body.unwrap().object,
                Target::Appl(
                    Selector::None,
                    ApplPayload::Pari {
                        channel: Channel::Channel1,
                        data: ApplPariParameter::MonitorSource { source },
                    },
                )
            );
        }

        assert!(
            param_to_msg(
                7,
                ParameterValue::Choice(ChoiceParameters::MonitorSource(), 3),
            )
            .is_err()
        );
    }

    #[test]
    fn serializes_mix_mono_buttons() {
        for bus in 0..=2 {
            for enabled in [false, true] {
                let message = param_to_msg(
                    7,
                    ParameterValue::Bool(BoolParameters::MixMono(bus), enabled),
                )
                .unwrap();
                let bytes = message.to_bytes().unwrap();

                assert_eq!(&bytes[12..16], &100_u32.to_le_bytes());
                assert_eq!(&bytes[16..20], &(bus as u32).to_le_bytes());
                assert_eq!(&bytes[20..24], b"iraP");
                assert_eq!(&bytes[28..32], &0_u32.to_le_bytes());
                assert_eq!(&bytes[32..36], &8_u32.to_le_bytes());
                assert_eq!(&bytes[36..40], &u32::from(enabled).to_le_bytes());
            }
        }

        assert!(param_to_msg(7, ParameterValue::Bool(BoolParameters::MixMono(3), true),).is_err());
    }

    #[test]
    fn serializes_output_and_blend_knobs() {
        for (parameter, parameter_id, value) in [
            (FloatParameters::HeadphonesGain(), 1_u32, -12.5),
            (FloatParameters::MainGain(), 2_u32, -6.0),
            (FloatParameters::MonitorMix(), 10_u32, 0.25),
        ] {
            let bytes = param_to_msg(7, ParameterValue::Float(parameter, value))
                .unwrap()
                .to_bytes()
                .unwrap();

            assert_eq!(&bytes[12..16], b"lppA");
            assert_eq!(&bytes[20..24], b"araP");
            assert_eq!(&bytes[28..32], &0_u32.to_le_bytes());
            assert_eq!(&bytes[32..36], &parameter_id.to_le_bytes());
            assert_eq!(&bytes[36..40], &value.to_le_bytes());
        }

        assert!(
            param_to_msg(7, ParameterValue::Float(FloatParameters::MonitorMix(), 1.1),).is_err()
        );
    }

    #[test]
    fn serializes_normalized_pan() {
        for (channel, value) in [(0, 0.0), (1, 0.5), (0, 1.0)] {
            let message = param_to_msg(
                7,
                ParameterValue::Float(FloatParameters::Pan(channel), value),
            )
            .unwrap();
            let bytes = message.to_bytes().unwrap();

            assert_eq!(&bytes[12..16], &0_u32.to_le_bytes());
            assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
            assert_eq!(&bytes[20..24], b"araP");
            assert_eq!(&bytes[28..32], &(channel as u32).to_le_bytes());
            assert_eq!(&bytes[32..36], &2_u32.to_le_bytes());
            assert_eq!(&bytes[36..40], &value.to_le_bytes());

            assert_eq!(
                message.body.unwrap().object,
                Target::Default(
                    Selector::None,
                    DefaultPayload::Para {
                        channel: channel.into(),
                        data: DefaultParaParameter::Pan { value },
                    },
                )
            );
        }

        assert!(param_to_msg(7, ParameterValue::Float(FloatParameters::Pan(0), 1.1),).is_err());
    }

    #[test]
    fn serializes_reverb_send() {
        for (channel, value) in [(0, -96.0), (1, -24.5), (0, 0.0)] {
            let message = param_to_msg(
                7,
                ParameterValue::Float(FloatParameters::ReverbSend(channel), value),
            )
            .unwrap();
            let bytes = message.to_bytes().unwrap();

            assert_eq!(&bytes[12..16], &0_u32.to_le_bytes());
            assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
            assert_eq!(&bytes[20..24], b"araP");
            assert_eq!(&bytes[28..32], &(channel as u32).to_le_bytes());
            assert_eq!(&bytes[32..36], &1_u32.to_le_bytes());
            assert_eq!(&bytes[36..40], &value.to_le_bytes());

            assert_eq!(
                message.body.unwrap().object,
                Target::Default(
                    Selector::None,
                    DefaultPayload::Para {
                        channel: channel.into(),
                        data: DefaultParaParameter::ReverbSend { value },
                    },
                )
            );
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

    #[test]
    fn serializes_mix_faders_as_bus_parameters() {
        let value = -12.5;
        for bus in 0..=2 {
            for source in 0..=5 {
                let bytes = param_to_msg(
                    7,
                    ParameterValue::Float(FloatParameters::MixFader(bus, source.into()), value),
                )
                .unwrap()
                .to_bytes()
                .unwrap();

                assert_eq!(&bytes[8..12], b"PteS");
                assert_eq!(&bytes[12..16], &100_u32.to_le_bytes());
                assert_eq!(&bytes[16..20], &(bus as u32).to_le_bytes());
                assert_eq!(&bytes[20..24], b"araP");
                assert_eq!(&bytes[28..32], &0_u32.to_le_bytes());
                assert_eq!(&bytes[32..36], &(source as u32).to_le_bytes());
                assert_eq!(&bytes[36..40], &value.to_le_bytes());
            }
        }
    }

    #[test]
    fn startup_refresh_reads_every_unsynchronized_control() {
        let messages = startup_refresh_messages(240);

        assert_eq!(messages.len(), 31);
        for (offset, message) in messages.iter().enumerate() {
            assert_eq!(message.nonce, 240_u8.wrapping_add(offset as u8));
            assert!(matches!(message.body.as_ref().unwrap().verb, Verb::GetP));
        }
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message.body.as_ref().unwrap().object,
                    Target::Default(_, DefaultPayload::Para { .. })
                ))
                .count(),
            4
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message.body.as_ref().unwrap().object,
                    Target::Opt(_, OptPayload::Pari { .. })
                ))
                .count(),
            2
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message.body.as_ref().unwrap().object,
                    Target::Appl(_, ApplPayload::Para { .. })
                ))
                .count(),
            3
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message.body.as_ref().unwrap().object,
                    Target::Appl(_, ApplPayload::Pari { .. })
                ))
                .count(),
            1
        );
    }

    #[test]
    fn mixer_refresh_reads_all_faders_and_modes() {
        let messages = mixer_refresh_messages(250);

        assert_eq!(messages.len(), 21);
        for (offset, message) in messages.iter().enumerate() {
            assert_eq!(message.nonce, 250_u8.wrapping_add(offset as u8));
            assert!(matches!(message.body.as_ref().unwrap().verb, Verb::GetP));
        }
    }
}
