use crate::device_state::{
    BoolParameters, ChoiceParameters, Command, FloatParameters, ParameterValue, State,
    jast_parameter_values,
};
use binrw::BinRead;
use nusb::{MaybeFuture, list_devices};
use pcap::Device;
use presonus_revalator::protocol::payload::{
    ApplParaParameter, ApplPariParameter, ApplPayload, BusParaParameter, BusPariParameter,
    BusPayload, DefaultParaParameter, DefaultPayload, OptPariParameter, OptPayload,
};
use presonus_revalator::protocol::protocol::{Channel, Message, Target, Verb};
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

    // This backend cannot send commands to the device. Drain them independently
    // so controls never block while packet capture is idle.
    std::thread::spawn(move || while update_rx.recv().is_ok() {});

    std::thread::spawn(move || {
        while let Ok(packet) = capture.next_packet() {
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
    if matches!(body.verb, Verb::GetP) {
        return None;
    }

    if !matches!(
        &body.object,
        Target::Appl(
            _,
            ApplPayload::AskJaSt | ApplPayload::JaSt(_) | ApplPayload::MemP { .. }
        )
    ) {
        println!("{body:?}");
    }

    match body.object {
        Target::Appl(_, ApplPayload::JaSt(payload)) => {
            Some(State::UpdateMany(jast_parameter_values(payload)))
        }
        Target::Appl(_, ApplPayload::Para { channel, data }) => match data {
            ApplParaParameter::Gain { value } => Some(State::Update(ParameterValue::Float(
                FloatParameters::Gain(channel as u8),
                value,
            ))),
            ApplParaParameter::Preset { enabled } => Some(State::Update(ParameterValue::Bool(
                BoolParameters::PresetEnabled(channel as u8),
                enabled,
            ))),
            ApplParaParameter::PhonesOut { value } => Some(State::Update(ParameterValue::Float(
                FloatParameters::HeadphonesGain(),
                value,
            ))),
            ApplParaParameter::MainOut { value } => Some(State::Update(ParameterValue::Float(
                FloatParameters::MainGain(),
                value,
            ))),
            ApplParaParameter::Blend { value } => Some(State::Update(ParameterValue::Float(
                FloatParameters::MonitorMix(),
                value,
            ))),
            ApplParaParameter::OutputDelay { .. } | ApplParaParameter::Unknown { .. } => None,
        },
        Target::Appl(
            _,
            ApplPayload::Pari {
                channel: Channel::Channel1,
                data: ApplPariParameter::MonitorSource { source },
            },
        ) => Some(State::Update(ParameterValue::Choice(
            ChoiceParameters::MonitorSource(),
            source as u32,
        ))),
        Target::Appl(_, ApplPayload::Pari { channel, data }) => match data {
            ApplPariParameter::PhantomPower { enabled } => Some(State::Update(
                ParameterValue::Bool(BoolParameters::PhantomPower(channel as u8), enabled),
            )),
            ApplPariParameter::ChannelLink { enabled } => Some(State::Update(
                ParameterValue::Bool(BoolParameters::ChannelLink(), enabled),
            )),
            ApplPariParameter::MonitorSource { .. }
            | ApplPariParameter::DelayChannel { .. }
            | ApplPariParameter::DualPreset { .. }
            | ApplPariParameter::Preset { .. }
            | ApplPariParameter::Unknown { .. } => None,
            ApplPariParameter::VoiceEffects { .. } => None,
        },
        Target::Default(_, DefaultPayload::Para { channel, data }) => match data {
            DefaultParaParameter::Pan { value } => Some(State::Update(ParameterValue::Float(
                FloatParameters::Pan(channel as u8),
                value,
            ))),
            DefaultParaParameter::ReverbSend { value } => Some(State::Update(
                ParameterValue::Float(FloatParameters::ReverbSend(channel as u8), value),
            )),
            DefaultParaParameter::Unknown { .. } => None,
        },
        Target::Bus(bus, BusPayload::Para { data, .. }) => {
            let (source, value) = match data {
                BusParaParameter::Playback { value } => (0, value),
                BusParaParameter::VirtualA { value } => (1, value),
                BusParaParameter::VirtualB { value } => (2, value),
                BusParaParameter::Channel1 { value } => (3, value),
                BusParaParameter::Channel2 { value } => (4, value),
                BusParaParameter::Reverb { value } => (5, value),
                BusParaParameter::Unknown { .. } => return None,
            };
            Some(State::Update(ParameterValue::Float(
                FloatParameters::MixFader(bus as u8, source.into()),
                value,
            )))
        }
        Target::Bus(
            bus,
            BusPayload::Pari {
                channel: Channel::Channel1,
                data: BusPariParameter::MixMono { enabled },
            },
        ) => Some(State::Update(ParameterValue::Bool(
            BoolParameters::MixMono(bus as u8),
            enabled,
        ))),
        Target::Bus(_, BusPayload::Pari { .. }) => None,
        Target::Opt(
            channel,
            OptPayload::Pari {
                channel: Channel::Channel1,
                data: OptPariParameter::ProcessingOrder { order },
            },
        ) => Some(State::Update(ParameterValue::Choice(
            ChoiceParameters::ProcessingOrder(channel as u8),
            order as u32,
        ))),
        Target::Opt(_, OptPayload::Pari { .. }) => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use presonus_revalator::protocol::payload::ProcessingOrder;
    use presonus_revalator::protocol::protocol::{Body, Bus, Selector, Verb};

    fn observed_state(object: Target) -> Option<State> {
        message_to_state(Message {
            body: Some(Body {
                verb: Verb::SetP,
                object,
            }),
            ..Default::default()
        })
    }

    fn observed_parameter(object: Target) -> ParameterValue {
        let state = observed_state(object).unwrap();

        match state {
            State::Update(parameter) => parameter,
            unexpected => panic!("expected one parameter update, got {unexpected:?}"),
        }
    }

    #[test]
    fn observes_processing_order() {
        let parameter = observed_parameter(Target::Opt(
            Channel::Channel2,
            OptPayload::Pari {
                channel: Channel::Channel1,
                data: OptPariParameter::ProcessingOrder {
                    order: ProcessingOrder::EqFirst,
                },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Choice(ChoiceParameters::ProcessingOrder(1), 1)
        );
    }

    #[test]
    fn observes_monitor_source() {
        let parameter = observed_parameter(Target::Appl(
            Selector::None,
            ApplPayload::Pari {
                channel: Channel::Channel1,
                data: ApplPariParameter::MonitorSource { source: Bus::MixB },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Choice(ChoiceParameters::MonitorSource(), 2)
        );
    }

    #[test]
    fn observes_output_and_blend_values() {
        for (data, expected) in [
            (
                ApplParaParameter::PhonesOut { value: -12.5 },
                ParameterValue::Float(FloatParameters::HeadphonesGain(), -12.5),
            ),
            (
                ApplParaParameter::MainOut { value: -6.0 },
                ParameterValue::Float(FloatParameters::MainGain(), -6.0),
            ),
            (
                ApplParaParameter::Blend { value: 0.25 },
                ParameterValue::Float(FloatParameters::MonitorMix(), 0.25),
            ),
        ] {
            let parameter = observed_parameter(Target::Appl(
                Selector::None,
                ApplPayload::Para {
                    channel: Channel::Channel1,
                    data,
                },
            ));
            assert_eq!(parameter, expected);
        }
    }

    #[test]
    fn observes_mix_mono() {
        let parameter = observed_parameter(Target::Bus(
            Bus::MixA,
            BusPayload::Pari {
                channel: Channel::Channel1,
                data: BusPariParameter::MixMono { enabled: true },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Bool(BoolParameters::MixMono(1), true)
        );
    }

    #[test]
    fn ignores_monitor_source_on_other_application_target() {
        assert!(
            observed_state(Target::Appl(
                Selector::None,
                ApplPayload::Pari {
                    channel: Channel::Channel2,
                    data: ApplPariParameter::MonitorSource {
                        source: Bus::MainMix,
                    },
                },
            ))
            .is_none()
        );
    }

    #[test]
    fn ignores_mono_on_other_target() {
        assert!(
            observed_state(Target::Bus(
                Bus::MainMix,
                BusPayload::Pari {
                    channel: Channel::Channel2,
                    data: BusPariParameter::MixMono { enabled: true },
                },
            ))
            .is_none()
        );
    }

    #[test]
    fn observes_reverb_send() {
        let parameter = observed_parameter(Target::Default(
            Selector::None,
            DefaultPayload::Para {
                channel: Channel::Channel1,
                data: DefaultParaParameter::ReverbSend { value: -12.5 },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Float(FloatParameters::ReverbSend(0), -12.5)
        );
    }

    #[test]
    fn ignores_getp_fader_request_values() {
        let state = message_to_state(Message {
            body: Some(Body {
                verb: Verb::GetP,
                object: Target::Bus(
                    Bus::MainMix,
                    BusPayload::Para {
                        channel: Channel::Channel1,
                        data: BusParaParameter::Playback { value: f32::NAN },
                    },
                ),
            }),
            ..Default::default()
        });

        assert!(state.is_none());
    }

    #[test]
    fn observes_mix_fader() {
        let parameter = observed_parameter(Target::Bus(
            Bus::MixA,
            BusPayload::Para {
                channel: Channel::Channel1,
                data: BusParaParameter::VirtualB { value: -12.5 },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Float(FloatParameters::MixFader(1, 2), -12.5)
        );
    }

    #[test]
    fn observes_pan() {
        let parameter = observed_parameter(Target::Default(
            Selector::None,
            DefaultPayload::Para {
                channel: Channel::Channel2,
                data: DefaultParaParameter::Pan { value: 0.75 },
            },
        ));

        assert_eq!(
            parameter,
            ParameterValue::Float(FloatParameters::Pan(1), 0.75)
        );
    }
}
