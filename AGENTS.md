# AGENTS.md

## Project overview

This repository is an early-stage Rust control application and reverse-engineering project for the **PreSonus Revelator IO24**.

The project has three related goals:

1. Document the device's vendor-specific USB control protocol from reproducible evidence.
2. Provide a typed Rust implementation of that protocol and a native GUI for controlling and monitoring the device.
3. Provide reusable `egui` audio widgets such as meters, knobs, and faders.

This is a control-plane application, not an audio driver. USB audio streaming, firmware updates, and internal audio sample formats are outside the current scope.

The implementation and `PROTOCOL.md` are both evolving. Do not infer device behavior from names, conventional audio behavior, or another PreSonus product. Treat unverified semantics as unknown or provisional until supported by captures or other primary evidence.

## Repository layout

The repository root is both the main library package and a Cargo workspace.

### `presonus_revalator`

Manifest: `Cargo.toml`

The root library contains the binary protocol representation:

- `src/lib.rs` — public library root.
- `src/protocol/protocol.rs` — outer message framing, verbs, selectors, targets, buses, and channels.
- `src/protocol/payload.rs` — typed payloads, parameter IDs, `JaSt`, bitfields, DSP blocks, and partial persistent-state parsing.

This layer should describe wire data and protocol concepts. It does not currently provide a complete high-level device client.

### `ui`

Manifest: `src/ui/Cargo.toml`

The native `eframe` application contains:

- `src/ui/src/main.rs` — startup, generic presentation, event reduction, and command dispatch.
- `src/ui/src/device_state.rs` — the authoritative device/UI model: parameter identities, ranges, presentation metadata, group topology, and `JaSt` mapping.
- `src/ui/src/state.rs` — macros for typed parameter families and their specifications.
- `src/ui/src/device.rs` — active `nusb` backend for a real device.
- `src/ui/src/device_usbmon.rs` — passive Linux `usbmon`/pcap observation backend.

`main.rs` should remain a generic presentation layer. Device-specific decisions—units, ranges, notches, meter channels, clipping sources, visibility, labels, and input semantics—belong in `device_state.rs`.

### `egui-audio-controls`

Manifest: `src/audio-controls/Cargo.toml`

The reusable widget crate contains:

- `src/audio-controls/src/lib.rs` — public API and semantic overview.
- `src/audio-controls/src/control.rs` — behavior shared by knobs and faders: values, formatting, direct entry, keyboard/wheel input, notches, and reset.
- `src/audio-controls/src/scale.rs` — linear, logarithmic, and audio-taper mappings plus amplitude/dB conversion.
- `src/audio-controls/src/knob.rs` — rotary interaction and painting.
- `src/audio-controls/src/fader.rs` — vertical fader interaction and painting.
- `src/audio-controls/src/meter.rs` — single/multi-channel meters, IEC-style deflection, clipping indicators, and peak hold/decay.
- `src/audio-controls/README.md` — public usage examples and scale semantics.

Keep widget behavior device-independent. Device-specific presentation configuration should be supplied through the widget APIs.

### Protocol and evidence documentation

- `PROTOCOL.md` — normative protocol draft and unresolved research gaps.
- `REVERSE_ENGINEERING.md` — required capture and evidence workflow.
- `tools/extract_dspusbdevice_firmware.sh` — reproducible extraction of known firmware images from a supported DLL.
- `captures/` — ignored packet captures used as primary evidence.
- `artifacts/` — ignored generated/reverse-engineering artifacts.

## Architecture and data flow

### Real-device flow

1. `main.rs` initializes the selected backend.
2. `device.rs` discovers the Revelator IO24, claims its vendor interface, and opens its bulk endpoints.
3. A scheduler emits initialization and refresh commands.
4. A USB worker translates typed UI commands into protocol messages, sends one message, then waits for its response.
5. Parsed responses become typed state updates.
6. `main.rs` drains updates into `EnumMap`-backed live state and renders the model declared by `device_state.rs`.

The known transport profile is:

- VID: `0x194f`
- PID: `0x0422`
- Interface: `5`
- Alternate setting: `1`
- Host-to-device endpoint: `0x01`
- Device-to-host endpoint: `0x81`

### Passive capture flow

The `usbmon` backend observes Linux USB traffic through pcap. It parses applicable host requests and device replies into the same typed state used by the real backend. UI commands are intentionally discarded because this backend is passive.

### UI model versus live state

`device_state.rs` separates presentation metadata from live values:

- Typed parameter enums identify values.
- Parameter specifications define valid ranges.
- `Parameter` presentation specs define how those values are shown and controlled.
- `DeviceState` in `main.rs` stores current values in one `EnumMap` per parameter family.
- `ParameterValue` is the typed boundary used for backend commands and state updates.

When adding a control, update both directions where applicable:

1. Device/model identity and presentation.
2. UI parameter to protocol message.
3. Protocol response or capture to UI parameter update.
4. Serialization and observation tests.

## Protocol rules

Treat `PROTOCOL.md` as the normative target, while remembering that it is still a draft.

- Numeric fields are little-endian unless explicitly documented otherwise.
- Four-character tags are little-endian integers and therefore look reversed in byte-oriented ASCII views.
- Preserve the distinction between **defined**, **reserved**, **unknown**, and **provisional** fields.
- Preserve unknown values when modifying a larger object.
- Do not synthesize provisional DSP or persistent-state fields without a compatible captured template or a fully established conversion.
- Validate framing lengths, typed-object lengths, protocol ID, direction, nonce, and status where the transaction layer has enough context.
- Compound operations consist of separate messages and acknowledgements; do not collapse them into one assumed operation.
- Preserve unsupported/opaque payloads where possible instead of rejecting future device fields unnecessarily.

When new capture evidence changes a protocol claim, update every affected location together:

- field/body definition;
- range and invalid values;
- read and write form;
- acknowledgement ordering;
- summary table;
- unresolved research gaps;
- Rust payload comments and tests.

Do not update protocol claims from UI behavior alone.

## Reverse-engineering workflow

Follow `REVERSE_ENGINEERING.md` for protocol investigations.

1. Change one control at a time.
2. Record reversible transitions and all enumeration values.
3. Preserve frame number and relative time in analysis.
4. Pair endpoint `0x01` requests with endpoint `0x81` replies by nonce.
5. Separate direct writes from acknowledgements, periodic `JaSt` polling, synchronization writes, DSP copies, and delayed `MemP` state.
6. Compare dwords as both `u32` and `f32` until representation is established.
7. Require both boolean directions or all enum values before assigning semantics.
8. State absence narrowly: “not identified in this capture,” not “does not exist.”

Useful documented commands include:

```sh
tshark -r captures/example.pcapng -q -z io,phs
```

```sh
tshark -r captures/example.pcapng \
  -Y 'usb.capdata' \
  -T fields \
  -e frame.number -e frame.time_relative \
  -e usb.endpoint_address -e usb.capdata
```

## UI conventions

- Put device-specific presentation facts in `device_state.rs`, not `main.rs`.
- Prefer explicit presentation structs and named fields over positional tuples.
- Give multi-channel meter channels names when they have meaningful identities.
- Represent optional clipping sources explicitly; a channel without a clipping source must not render an empty clipping indicator.
- Keep normalized percentages, linear amplitude, direct dB values, and gain-reduction values semantically distinct.
- Main and headphone output controls are normalized percentages (`0.0..=1.0`).
- Do not infer units from a parameter name in the renderer.
- Keep linked/unlinked visibility in the model.
- Avoid blocking or flooding the bounded device-command channel during high-rate interactions; prefer coalescing if control update rates become a problem.

## Widget conventions

- Preserve the builder-style public API.
- Keep shared knob/fader behavior in `control.rs`; keep painting and physical drag interaction in the individual widgets.
- Keyboard and wheel increments are value-domain increments.
- Arrow keys and wheel input operate while the widget is hovered.
- Shift selects the configured fine increment.
- Wheel interaction uses raw wheel events; do not reintroduce smoothed/tweened scroll deltas for value changes.
- Keyboard stepping intentionally bypasses magnetic snapping so a value can leave a notch.
- Use `ValueScale::Logarithmic` for strictly positive multiplicative quantities such as frequency.
- Use the audio taper for stored-dB faders.
- `LevelMeter::new` accepts linear amplitude; `LevelMeter::from_db` accepts direct dB values.
- A `LevelMeter` may contain one or arbitrarily many adjacent channels.
- Keep clipping attachment optional per channel.
- Update crate docs and README examples whenever public semantics change.

## Feature flags and running the application

The `ui` crate requires exactly one backend:

- `real` — default, active `nusb` device control.
- `usbmon` — passive pcap capture.

Do not use `--all-features`; enabling both backends intentionally fails compilation.

Run the real backend:

```sh
cargo run -p ui
```

Run the passive backend:

```sh
cargo run -p ui --no-default-features --features usbmon
```

The passive backend currently expects the capture interface configured in `device_usbmon.rs` and requires libpcap/usbmon access. The real backend requires OS permission to claim the device interface.

## Build and test commands

Run commands from the repository root.

```sh
cargo check --workspace
cargo test --workspace
```

Test individual packages while iterating:

```sh
cargo test -p presonus_revalator
cargo test -p egui-audio-controls
cargo test -p ui
cargo test -p ui --no-default-features --features usbmon
```

Check the reusable widget crate strictly:

```sh
cargo fmt -p egui-audio-controls -- --check
cargo clippy -p egui-audio-controls --all-targets -- -D warnings
```

Because backend features are mutually exclusive, test the default and `usbmon` configurations separately rather than using `--all-features`.

When a pre-existing test fails, verify whether the failure predates the current work and report it separately. Do not silently change unrelated protocol behavior merely to make a test green.

## Working practices for agents

- Read the relevant model, protocol documentation, and both translation directions before modifying behavior.
- Match existing Rust naming and formatting.
- Prefer focused tests at pure conversion/model seams.
- Run the affected package tests frequently and the full workspace suite before declaring work complete.
- Use `jj` for version-control inspection in this repository.
- Do not create commits unless the user explicitly requests one.
- Do not conduct a code review until the user explicitly says the feature is complete and requests review.
- Ask before making changes based on uncertain device semantics.
- Avoid changing unrelated files or cleaning up concurrent user work.

## Important current limitations

This remains prototype software around a reverse-engineered protocol. In particular:

- Many protocol and DSP fields remain unknown or provisional.
- Persistent preset/state handling is partial.
- Hardware recovery, reconnect behavior, and user-facing backend errors are incomplete.
- There is no mock/offline backend or capture-replay test harness yet.
- USB audio streaming is intentionally not implemented.

Treat these as boundaries, not invitations for opportunistic scope expansion. Discuss larger protocol, backend, or state-management changes with the user before implementing them.

## Recommended reading order

1. `PROTOCOL.md`
2. `REVERSE_ENGINEERING.md`
3. `src/ui/src/device_state.rs`
4. `src/ui/src/main.rs`
5. `src/ui/src/device.rs`
6. `src/protocol/protocol.rs`
7. `src/protocol/payload.rs`
8. `src/ui/src/device_usbmon.rs`
9. `src/audio-controls/README.md`
10. `src/audio-controls/src/lib.rs`
11. `src/audio-controls/src/control.rs`
12. `src/audio-controls/src/meter.rs`
