# USB Protocol Reverse-Engineering Workflow

This document describes a repeatable workflow for extending `PROTOCOL.md` from
USB captures. It is intentionally tool-light: Wireshark/TShark and a short
script are sufficient for most comparisons.

## 1. Design the capture

Change one control at a time and record the exact sequence in the capture name
or an adjacent note. Prefer reversible transitions that expose both encodings:

```text
control: baseline -> value A -> value B -> baseline
```

For enumerations, visit every value before returning to the baseline. Wait long
enough after each action to distinguish the write burst, acknowledgement, state
polling, and any delayed `MemP` commit. Record the device firmware, Universal
Control version, sample rate, and initial channel/preset state whenever they are
known.

Use a separate capture for compound actions such as linking channels or loading
a preset. These actions can cause synchronization writes unrelated to the
control that initiated them.

## 2. Extract USB payloads

Confirm that TShark can read the capture:

```sh
tshark -r captures/example.pcapng -q -z io,phs
```

List frames carrying captured USB data:

```sh
tshark -r captures/example.pcapng \
  -Y 'usb.capdata' \
  -T fields \
  -e frame.number -e frame.time_relative \
  -e usb.endpoint_address -e usb.capdata
```

Endpoint `0x01` contains host requests and endpoint `0x81` contains device
replies for the control interface. Keep frame number and relative time in every
intermediate report; they make results independently reproducible.

To inspect selected frames only:

```sh
tshark -r captures/example.pcapng \
  -Y 'frame.number in {2033,2035,2039}' \
  -T fields \
  -e frame.number -e frame.time_relative \
  -e usb.endpoint_address -e usb.capdata
```

## 3. Decode the common envelope

Decode each endpoint-`0x01` payload using the layouts in Sections 4 and 5 of
`PROTOCOL.md`:

1. validate `MessageHeader.total_length`;
2. decode `operation`, `object`, and `selector`;
3. decode the typed-object tag and length;
4. split the data body into little-endian dwords;
5. display each dword as both `u32` and `f32` until its representation is known;
6. pair the request with the endpoint-`0x81` response having the same nonce.

Do not search packet bytes as ordinary ASCII without accounting for the
little-endian tag convention. For example, `SetP/Appl/Pari` appears on the wire
with tag bytes `50 74 65 53`, `6c 70 70 41`, and `69 72 61 50`.

## 4. Separate traffic classes

Classify requests before interpreting values:

- `GetP/Appl/JaSt`: periodic state and meter polling;
- header-only endpoint-`0x81` messages: write acknowledgements;
- small `Para` or `Pari` writes: likely scalar or enumerated controls;
- effect-specific blocks: real-time DSP updates;
- `MemP/Stat`: delayed named-state synchronization;
- unknown `MemP` variants: possible store or management operations.

A single UI action may generate several classes. The earliest small write is a
strong control candidate, while later DSP copies and `MemP` messages may be
consequences rather than alternate encodings of that control.

## 5. Correlate transitions

For each recorded action, compare the quiet interval before it with the first
write burst after it. A useful result table contains:

| Action | Frame/time | Operation/object/type | Selector | Body | Reply | Readback |
|---|---|---|---:|---|---|---|

Require both directions of a boolean transition before assigning `0` and `1`.
For an enumeration, require all observed values. Correlate `JaSt` replies by
comparing dwords and bytes before and after the action; ignore continuously
changing meter floats unless the candidate field settles to a stable state.

Compound operations require extra care. For channel linking, for example,
distinguish the two direct link-membership writes from subsequent pan, option,
and DSP-state synchronization. Do not assign meanings to synchronization-only
parameter IDs unless a separate capture isolates them.

## 6. Inspect persistent state

For `MemP`, first compare the fixed binary prefix and locate the UBJSON object;
do not assume that all variants use the same header. Decode UBJSON while
preserving:

- original key order;
- unknown keys and class identifiers;
- integer marker widths;
- big-endian floating-point values;
- trailing padding.

Use `preset_name` to identify a completed preset application. Do not infer a
preset number from the preceding block count or sequence. To characterize
custom slots, capture stores and recalls for every slot using otherwise
identical state; a one-slot capture cannot isolate the slot field.

## 7. Confidence rules

Classify findings using the terminology in Section 2 of `PROTOCOL.md`:

- **Defined:** isolated by controlled transitions and reproducible in the
  corresponding request or stable readback;
- **Provisional:** correlated with the action but confounded by other writes, or
  only partly decoded;
- **Unknown:** observed bytes with no assigned semantics;
- **Reserved:** only when a required transmitted value has been established.

Absence is also a finding, but state it narrowly. If no vendor-interface write
or stable `JaSt` change appears, report that no encoding was identified in that
capture; do not conclude that the feature has no USB representation anywhere.

## 8. Update the specification

When adding a control to `PROTOCOL.md`:

1. define the complete envelope, selector, typed length, and body fields;
2. state value ranges and invalid values;
3. describe acknowledgement ordering for compound updates;
4. identify any readback separately from the write encoding;
5. preserve unresolved fields and list them in Section 11;
6. add the control to the summary table;
7. avoid frame numbers in normative text, keeping capture-specific evidence in
   analysis notes or commit history.

Before finishing, verify lengths arithmetically, check tag wire order, confirm
both request and acknowledgement nonces, and search the document for older
claims contradicted by the new capture.
