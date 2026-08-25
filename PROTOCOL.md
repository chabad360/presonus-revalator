# PreSonus Revelator IO24 Control Protocol Specification

Version 0.4-draft

## 1. Scope

This document specifies the USB userspace control protocol used by the
PreSonus Revelator IO24. It covers:

- USB transport parameters;
- message framing and request/reply behavior;
- application operations and typed objects;
- preset enable control;
- channel DSP controls;
- state and meter readback;
- persistent processing-state serialization.

USB Audio streaming, device firmware update, and the internal audio sample
format are outside the scope of this specification.

Fields whose semantics are not specified are named `reserved`, `unknown`, or
`provisional`. Section 11 defines the corresponding research requirements.
Implementations shall preserve unspecified field values when modifying a
larger object unless the object definition explicitly permits replacement.

## 2. Conventions

The key words **shall**, **should**, and **may** describe required,
recommended, and optional behavior respectively.

Unless otherwise stated:

- integer fields are unsigned;
- integers and IEEE-754 values are little-endian;
- offsets are relative to the beginning of the containing structure;
- lengths are in bytes;
- four-character tags are encoded as little-endian 32-bit integers;
- structure layouts contain no implicit padding.

Field-status terminology is defined as follows:

- **Defined** fields have a normative encoding and may be generated directly.
- **Reserved** fields shall be transmitted with the value required by the
  structure definition and shall otherwise be ignored by receivers.
- **Unknown** fields have no assigned semantic meaning and shall be preserved.
- **Provisional** fields have a partial semantic mapping and shall be generated
  only from a compatible template or a fully specified conversion.

Four-character tags appear reversed when their wire bytes are viewed as ASCII.
For example, the tag `GetP`, whose integer value is `0x47657450`, is transmitted
as `50 74 65 47` (`PteG`).

## 3. USB Transport

The control protocol uses a vendor-specific USB interface and bulk transfers.

| Property | Value |
|---|---:|
| USB vendor ID | `0x194f` |
| USB product ID | `0x0422` |
| Interface number | `5` |
| Alternate setting | `1` |
| Host-to-device bulk endpoint | `0x01` |
| Device-to-host bulk endpoint | `0x81` |

The defined transport profile shall send one complete protocol message to
endpoint `0x01` per USB bulk transfer. The device shall return acknowledgements
and typed replies on endpoint `0x81`. This profile does not permit fragmented
or concatenated protocol messages.

The control protocol is independent of the USB Audio streaming endpoints.

## 4. Message Format

### 4.1 General layout

A message consists of an eight-byte message header followed by an optional
application frame.

```text
+----------------------+ offset 0
| Message header       | 8 bytes
+----------------------+ offset 8
| Application frame    | optional
+----------------------+
```

The 16-bit `total_length` field can represent messages up to 65,535 bytes. This
specification does not define a device acceptance limit below that value.

### 4.2 Message header

```c
#pragma pack(push, 1)
struct MessageHeader {
    uint16_t total_length;
    uint8_t  protocol_id;
    uint8_t  message_code;
    uint8_t  nonce;
    uint8_t  status;
    uint16_t reserved;
};
#pragma pack(pop)
```

| Offset | Size | Field | Description |
|---:|---:|---|---|
| `0x00` | 2 | `total_length` | Complete message length, including this header |
| `0x02` | 1 | `protocol_id` | Protocol version/identifier; shall be `0x01` |
| `0x03` | 1 | `message_code` | `0x01` for a request; `0x81` for a response |
| `0x04` | 1 | `nonce` | Transaction correlation value |
| `0x05` | 1 | `status` | Zero on requests and successful responses |
| `0x06` | 2 | `reserved` | Shall be transmitted as zero |

The minimum valid message length is eight bytes.

### 4.3 Application frame

Messages longer than eight bytes contain an application prefix followed by
one typed object.

```c
#pragma pack(push, 1)
struct ApplicationPrefix {
    uint32_t operation;
    uint32_t object;
    uint32_t selector;
};

struct TypedObject {
    uint32_t type;
    uint32_t total_length;
    uint8_t  data[];
};
#pragma pack(pop)
```

| Message offset | Size | Field | Description |
|---:|---:|---|---|
| `0x08` | 4 | `operation` | Operation tag |
| `0x0c` | 4 | `object` | Destination object tag |
| `0x10` | 4 | `selector` | Object-specific selector |
| `0x14` | 4 | `type` | Typed-object tag |
| `0x18` | 4 | `typed_length` | Typed-object size, including its eight-byte header |
| `0x1c` | variable | `data` | Typed-object data |

The following length relationships shall hold:

```text
typed_length = 8 + data_length
message.total_length = 8 + 12 + typed_length
message.total_length = 28 + data_length
```

### 4.4 Tags

Known operation tags are:

| Tag | Integer value | Wire bytes | Description |
|---|---:|---|---|
| `GetP` | `0x47657450` | `50 74 65 47` | Read an object/property |
| `SetP` | `0x53657450` | `50 74 65 53` | Write an object/property |
| `Rply` | `0x52706c79` | `79 6c 70 52` | Typed response |

Known object tags are:

| Tag | Description |
|---|---|
| `Appl` | Application state and application-level controls |
| `filt` | High-pass filter |
| `gate` | Gate/expander |
| `comp` | Compressor |
| `eq  ` | Four-band equalizer; tag contains two trailing spaces |
| `lim ` | Limiter; tag contains one trailing space |
| `opt ` | Application options, including processing-order options |

Known typed-object tags are:

| Tag | Description |
|---|---|
| `JaSt` | Application state and metering snapshot |
| `Para` | Small application parameter wrapper; value representation depends on its parameter ID |
| `Pari` | Small integer application parameter wrapper |
| `MemP` | Persistent/named processing state |
| `MBdf` | Extended filter implementation block |
| `Bqdf` | Biquad coefficient block |
| `Lfdf` | Provisional extended EQ/filter block |
| `MLff` | Provisional aggregate EQ/filter implementation block |
| `gate` | Gate processing block |
| `cpxt` | Compressor processing block |
| `lim ` | Limiter processing block |

## 5. Transactions

### 5.1 Nonce handling

The host shall assign a nonce to each request. The device shall copy that nonce
into the corresponding response.

The nonce is an eight-bit counter and may wrap from `0xff` to `0x00`. A host
shall not confuse a response with an older request after wraparound.

### 5.2 Write acknowledgement

A successful `SetP` transaction shall return a header-only response:

```text
total_length = 8
protocol_id  = 1
message_code = 0x81
nonce        = request nonce
status       = 0
reserved     = 0
```

Example acknowledgement for nonce `0x27`:

```text
08 00 01 81 27 00 00 00
```

### 5.3 Typed read response

A successful `GetP` transaction returns a complete application frame:

```text
operation = Rply
object    = the requested object
selector  = the requested selector
type      = the requested typed-object type
```

### 5.4 Status values

Status value `0` means success. Non-zero values are unspecified. A host shall
treat any non-zero status as a failed transaction and should retain the raw
status for diagnostics.

### 5.5 Input validation

A receiver shall reject a message when:

1. fewer than eight bytes are available;
2. `total_length < 8`;
3. fewer than `total_length` bytes are available;
4. `protocol_id != 1`;
5. `message_code` is unsupported;
6. a response nonce does not match the pending request;
7. a typed object's length is less than eight bytes;
8. the typed-object and outer lengths are inconsistent.

A USB transfer may contain bytes after the logical message. A receiver should
use `total_length` as the parsing boundary rather than requiring the transfer
length to equal the message length exactly.

### 5.6 Compound updates

One user operation may consist of several independent `SetP` transactions.
Each transaction has its own nonce and shall be acknowledged before the next
transaction in the sequence is sent.

The following sequences are defined by the channel-2 processing operations:

```text
Filter coefficient update:
    SetP / <effect> / MBdf
    acknowledge
    SetP / <effect> / Bqdf
    acknowledge

Gate or compressor parameter update:
    SetP / <effect> / gate-or-cpxt
    acknowledge

Compressor key-filter update:
    SetP / comp / cpxt
    acknowledge
    SetP / comp / MBdf
    acknowledge
```

After a write sequence, the host should resume `GetP/Appl/JaSt` polling. After
the state has settled, it may send a separate `SetP/Appl/MemP` snapshot.
The `MBdf` and `Bqdf` packets are not one combined USB message; they are
separate messages with separate acknowledgements.

During continuous parameter drags, the host repeats the same sequence for each
intermediate value. A driver shall not assume that one visual gesture maps to
one protocol packet.

## 6. Application State (`JaSt`)

### 6.1 Request and reply

The host obtains current state and meter values using:

```text
Request:  GetP / Appl / selector 0 / JaSt
Response: Rply / Appl / selector 0 / JaSt
```

The `JaSt` typed object is 220 (`0xdc`) bytes:

```text
8-byte typed-object header
212-byte data body
```

The data body consists of 53 little-endian dwords.

### 6.2 Body layout

The `JaSt` body shall use the following physical layout. The scalar region is
provisional; consumers shall not assign a signal name to an individual scalar
unless that mapping is defined by a compatible profile.

| Data offset | Dword indices | Size | Interpretation |
|---:|---:|---:|---|
| `0x00..0x94` | 0..37 | 152 | Scalar floating-point meter/state values |
| `0x98` | 38 | 4 | Channel-link/activity mask 1 |
| `0x9c` | 39 | 4 | Channel-link/activity mask 2 |
| `0xa0..0xac` | 40..43 | 16 | Four unknown state dwords |
| `0xb0` | 44 | 4 | Status flags |
| `0xb4..0xbc` | 45..47 | 12 | Three global floating-point values |
| `0xc0..0xcc` | 48..51 | 16 | Four per-channel floating-point values |
| `0xd0..0xd3` | 52 | 4 | Four state/event byte flags; bytes 0 and 1 contain per-channel flags |

The scalar region provides values for the following signal classes:

- playback L/R;
- virtual output A L/R;
- virtual output B L/R;
- analog input channels;
- main output L/R;
- stream mix A L/R;
- stream mix B L/R;
- gate, compressor, and limiter meters/reduction values.

The index-to-signal mapping is unspecified for any scalar not assigned by a
compatible profile.

### 6.3 Preset bypass status

The following `status_flags` bits are defined:

| Bit | Mask | Meaning when set |
|---:|---:|---|
| 5 | `0x00000020` | Channel 1 preset processing is disabled/bypassed |
| 6 | `0x00000040` | Channel 2 preset processing is disabled/bypassed |

Bit 0 (`0x00000001`) is unspecified and shall be preserved when the containing
state is rewritten.

### 6.4 Additional discrete state

For this profile, bytes 0 and 1 of dword 52, at data offsets `0xd0` and
`0xd1`, report channel 1 and channel 2 state flags respectively. Bit 0 of each
byte reports phantom-power state:

| Value | Meaning |
|---:|---|
| `0` | Phantom power disabled |
| `1` | Phantom power enabled |

In each channel activity dword at indices 38 and 39, mask `0x000000c0`
indicates that the corresponding channel is clipping. Other activity bits
remain unspecified.

Channel-link operations produce correlated changes in dword 40 and in
`status_flags` mask `0x00001000`. An unlinked capture contained dword-40 value
`3` with the mask clear; a linked capture contained value `4` with the mask
set. These mappings are provisional because the controller performed other
synchronization operations during the same transition.

`status_flags` mask `0x00000080` also changes during some preset applications.
Its meaning is unspecified.

### 6.5 Physical-control notification behavior

A physical preset-enable change is first observable in a subsequent `JaSt`
reply. A connected host application may then send the corresponding
`SetP/Appl/Para` value back to the device to synchronize application state.

This protocol profile defines no unsolicited device-to-host notification for
this operation. Implementations should poll `JaSt` when physical-control
changes must be detected.

## 7. Preset Enable Control (`Para`)

Preset processing for either input channel is controlled using:

```text
operation = SetP
object    = Appl
selector  = 0
type      = Para
typed length = 20
```

The 12-byte data body is:

```c
#pragma pack(push, 1)
struct PresetEnableData {
    uint32_t channel_index;
    uint32_t parameter_id;
    float    enabled;
};
#pragma pack(pop)
```

| Field | Required value |
|---|---|
| `channel_index` | `0` for channel 1; `1` for channel 2 |
| `parameter_id` | `4` |
| `enabled` | `0.0` to bypass; `1.0` to enable |

Values other than exactly `0.0` and `1.0` are invalid and shall not be
transmitted.

## 8. Channel DSP Control

### 8.1 Channel selector

Effect-specific messages include a selector in the application prefix. The
defined channel-2 transactions use selector `1`, matching the zero-based
channel index used by the `Para` preset-enable object:

```text
selector 0 = channel 1
selector 1 = channel 2
```

This selector convention applies only to the input-channel DSP objects defined
by this specification. Bus, output, and linked-channel selectors are outside
the defined address space.

### 8.2 Biquad coefficient object (`Bqdf`)

`Bqdf` has typed length 36 and data length 28. Its data body is exactly:

```c
#pragma pack(push, 1)
struct BiquadData {
    uint32_t unknown0;
    uint32_t section_index;
    float    coefficient[5];
};
#pragma pack(pop)
```

| Field | Size | Meaning |
|---|---:|---|
| `unknown0` | 4 | Reserved; shall be zero |
| `section_index` | 4 | Filter section or EQ band index |
| `coefficient[0..4]` | 20 | Five calculated filter coefficients |

The coefficient order and sign convention are unspecified. Implementations
shall treat the array as opaque values `c0..c4`. The values shall correspond to
the device's current sample rate; `MBdf` carries the sample-rate-specific
records.

For the high-pass filter, `section_index = 1`. For the equalizer,
`section_index = 0`, `1`, `2`, or `3` selects one of the four bands.

HPF bypass is represented by the identity vector:

```text
[1.0, 0.0, 0.0, 0.0, 0.0]
```

Enabling the HPF restores calculated coefficients. One defined 40 Hz setting
produces approximately:

```text
[0.996304, 1.992595, -1.992609, -0.992623, 0.996304]
```

In persistent state, `filter.hpf = 24` represents bypass and
`filter.hpf = 40` represents the defined 40 Hz setting. The value 24 is a
control sentinel; the accompanying identity coefficients establish that a
24 Hz filter is not active.

### 8.3 Extended coefficient object (`MBdf`)

`MBdf` has typed length 500 and data length 492. The first 104 bytes have a
stable sample-rate coefficient layout:

```c
#pragma pack(push, 1)
struct BiquadRateRecord {
    float coefficient[5];
    float sample_rate;
};

struct MBdfKnownPrefix {
    uint32_t unknown0;
    uint32_t unknown1;
    BiquadRateRecord record[4];
    uint8_t unknown_tail[388];
};
#pragma pack(pop)
```

The four records are transmitted in this order:

| Record | Sample rate |
|---:|---:|
| 0 | `44100.0` Hz |
| 1 | `48000.0` Hz |
| 2 | `88200.0` Hz |
| 3 | `96000.0` Hz |

The first two dwords are zero in the defined filter blocks. The first 104
bytes are therefore:

```text
offset 0x00: unknown0
offset 0x04: unknown1
offset 0x08: record 0 (24 bytes)
offset 0x20: record 1 (24 bytes)
offset 0x38: record 2 (24 bytes)
offset 0x50: record 3 (24 bytes)
```

The remaining 388 bytes have no portable schema. They may contain zeroes,
implementation metadata, and process-local values. An implementation shall
not interpret these bytes as portable pointers or construct them from host
addresses.

`MBdf` accompanies `Bqdf` for HPF, EQ, and compressor/gate key-filter updates.
An implementation shall preserve the unknown tail from a block produced by the
same device model and firmware revision. Cross-revision reuse is unspecified.

### 8.4 Gate/expander

The gate uses object tag `gate`, typed-object tag `gate`, and a 64-byte body
containing 16 dwords.

Known fields are:

| Dword | Type | Meaning |
|---:|---|---|
| 0 | `u32` | Reserved; shall be zero |
| 1, 2, 4, 5 | `f32` | Key-filter biquad values; dword 3 is zero |
| 6 | `f32` | Expander range scale, `10^(range_db/20)` |
| 7 | `f32` | Gate threshold scale, `10^(-threshold_db/20)` |
| 8 | `f32` | Attack, in seconds |
| 9 | `f32` | Release, in seconds |
| 10 | `f32` | Attack-derived coefficient |
| 11 | `f32` | Release-derived coefficient |
| 12 | `u32` | Provisional mode-dependent value; shall be preserved |
| 13 | `u32` | Enable: `0` disabled, `1` enabled |
| 14 | `u32` | Expander mode: `1` automatic, `0` manual |
| 15 | `u32` | Key listen: `0` off, `1` on |

Example threshold encodings:

| Threshold | Dword 7 value |
|---:|---:|
| `-45.4 dB` | approximately `186.2088` |
| `-47.5 dB` | approximately `237.1374` |
| `-61.36 dB` | approximately `1169.4994` |

The remaining dwords shall be preserved from a compatible block. Dword 12 is
mode-dependent, but this specification does not define its conversion from a
user-facing value.

Changing the gate key-filter frequency sends two messages in order:

```text
SetP / gate / MBdf
acknowledge
SetP / gate / gate
acknowledge
```

The `MBdf` carries the key-filter coefficients for all four supported sample
rates. The following `gate` block carries the active-channel values.

### 8.5 Compressor

The compressor uses object tag `comp`, typed-object tag `cpxt`, and a 56-byte
body containing 14 dwords.

The defined and provisional fields are:

| Dword | Type | Meaning |
|---:|---|---|
| 0 | `u32` | Reserved; shall be zero |
| 1, 2, 4, 5 | `f32` | Key-filter biquad values; dword 3 is zero |
| 6 | `f32` | Attack, in seconds |
| 7 | `f32` | Release, in seconds |
| 8 | `f32` | Ratio transform: `1 - 1/ratio` (`0.5` for 2:1; `0.75` for 4:1) |
| 9 | `f32` | Provisional soft-knee-derived value; shall be preserved unless a compatible conversion is available |
| 10 | `f32` | Threshold directly in dB |
| 11 | `f32` | Gain scale, `10^(gain_db/20)` |
| 12 | `u32` | Enable: `0` disabled, `1` enabled |
| 13 | `u32` | Key listen: `0` off, `1` on |

Threshold values are not converted to a linear scale. For example, a stored
threshold of `-15.4 dB` is transmitted as the float `-15.4` in dword 10.

Automatic mode shall be represented in `MemP/Stat` by `comp.automode`. This
specification defines no corresponding real-time `cpxt` encoding. Dword 9 is a
soft-knee-derived value; its conversion is unspecified.

Changing the compressor key-filter frequency sends two messages in the reverse
order used by the gate:

```text
SetP / comp / cpxt
acknowledge
SetP / comp / MBdf
acknowledge
```

The `cpxt` block contains the active compressor values and the `MBdf` block
contains the four sample-rate key-filter coefficient records. Changes to
threshold, ratio, attack, release, gain, or key-listen use only `cpxt`.

### 8.6 Four-band equalizer

The equalizer uses object tag `eq  `. Each band is represented by an `MBdf`
object followed by a `Bqdf` object.

The `Bqdf` body uses the `BiquadData` structure defined in Section 8.2. For EQ:

```text
unknown0 = 0
section_index = 0, 1, 2, or 3
```

Section indices shall map to bands as follows:

| Section index | EQ band |
|---:|---|
| `0` | Low |
| `1` | Low-mid |
| `2` | High-mid |
| `3` | High |

An EQ band change shall use this sequence, with a successful acknowledgement
after each request:

```text
SetP / eq   / MBdf   (typed length 500)
acknowledge
SetP / eq   / Bqdf   (typed length 36)
acknowledge
```

Global EQ bypass is implemented by sending the identity coefficient vector for
all four section indices. Enabling the EQ shall restore the active coefficient
vector for each band. Band enable, shelf mode, frequency, gain, and Q changes
shall be expressed by replacing coefficient vectors; these controls do not
define additional message types or selectors.

An edit to one band may send only that band's `MBdf`/`Bqdf` pair. There is no
requirement to resend all four bands for an ordinary parameter update.
A continuous parameter gesture may generate multiple update sequences.

The conversion from user-facing frequency, Q, gain, and band mode to the five
transmitted coefficients is unspecified. Implementations without a compatible
coefficient generator shall use coefficient blocks produced by a conforming
controller.

Preset transitions also use two provisional equalizer object types:

| Type | Typed length | Data length | Observed form |
|---|---:|---:|---|
| `Lfdf` | 44 | 36 | Nine-dword extended filter block |
| `MLff` | 660 | 652 | Aggregate filter implementation block |

Both types use object `eq  ` and the input-channel selector. They occur during
preset transitions that apply repeated, interpolated EQ coefficient updates.
Their body layouts and relationship to individual EQ bands are unspecified.
An implementation shall preserve these bodies as opaque values and shall not
generate them without a compatible template or fully specified conversion.

### 8.7 Limiter

The limiter uses object and typed-object tag `lim `. Its 16-byte body is:

```c
#pragma pack(push, 1)
struct LimiterData {
    uint32_t unknown0;
    uint32_t enabled;
    float    threshold_scale;
    float    smoothing;
};
#pragma pack(pop)
```

| Field | Meaning |
|---|---|
| `unknown0` | Reserved; shall be zero |
| `enabled` | `0` disabled; `1` enabled |
| `threshold_scale` | `10^(-threshold_db/20)` |
| `smoothing` | Provisional smoothing coefficient; `0.999673` for this profile |

Examples:

| Threshold | `threshold_scale` |
|---:|---:|
| `-26.84 dB` | approximately `21.9786` |
| `-24.88 dB` | approximately `17.5388` |
| `0 dB` | `1.0` |
| `-20 dB` | `10.0` |

The relationship between `smoothing`, time constant, and sample rate is
unspecified. Implementations using this profile shall transmit `0.999673`.

### 8.8 Generic `Para` parameter controls

Generic floating-point channel parameters shall use the following 12-byte
`Para` body. `PresetEnableData` is the parameter-ID-4 specialization of this
layout:

```c
struct GenericParameterData {
    uint32_t target_index;
    uint32_t parameter_id;
    uint32_t raw_value;
};
```

Generic parameter writes shall use `SetP`, selector `0`, and type `Para`.
`target_index` shall be the zero-based channel index. The following parameter
IDs are defined:

| Parameter ID | Object | Value representation |
|---:|---|---|
| `1` | `0x00000000` | Reverb send in dB; continuous changes may emit intermediate values |
| `2` | `0x00000000` | Pan normalized to `[0.0, 1.0]`; `0.0` is full left, `0.5` is center, and `1.0` is full right |
| `3` | `Appl` | Channel gain in dB |
| `4` | `Appl` | Preset processing enable, encoded as `f32` 0.0 or 1.0 |

For parameter IDs 1 through 4, `raw_value` shall contain a little-endian
IEEE-754 binary32 value. For channel 2, setting gain to 0 dB shall produce the
body `(1, 3, 0x00000000)`. Values outside the pan interval `[0.0, 1.0]` shall
not be transmitted. The fader parameter ID and value conversion are
unspecified. The fixed-size `mprm` object shall not be interpreted as a fader
update without a compatible schema.

### 8.9 Integer `Pari` controls

Integer and enumerated controls use a distinct 20-byte typed object whose wire
tag is `Pari`. Its 12-byte body is:

```c
struct IntegerParameterData {
    uint32_t target_index;
    uint32_t parameter_id;
    uint32_t value;
};
```

Defined writes use `SetP`, selector `0`, and the following mappings:

| Object | Parameter ID | `target_index` | Value |
|---|---:|---:|---|
| `Appl` | `0` | `0` or `1` | Per-channel phantom power: `0` off, `1` on |
| `Appl` | `9` | `0` or `1` | Channel-link membership: `0` unlinked, `1` linked |
| `Appl` | `11` | `0` | Monitor source: `0` Main, `1` Mix A, `2` Mix B |
| `opt ` | `0` | `0` | Processing order (`swapcompeq`): `0` or `1` |
| `0x00000064` | `8` | `0` | Mix Main Mono: `0` stereo, `1` mono |

To link input channels 1 and 2, the host shall write parameter ID 9 with value
`1` for both target indices, acknowledging each write. To unlink them, it shall
write value `0` for both target indices. A link transition may cause the
controller to synchronize pan, processing order, and complete DSP state; these
additional writes shall not be interpreted as part of the parameter-ID-9
encoding.

Phantom-power writes use the corresponding zero-based channel index. Both
channel flag bytes expose the same phantom-power readback bit.

Transactions with other parameter IDs, including observed
`SetP/Appl/Pari` bodies `(0, 12, 0)`, `(1, 12, 1)`, and `(1, 16, 2)`, have no
assigned control meaning and shall not be generated without a compatible
profile. Firmware-derived candidate IDs 18, 27, and 28 did not produce an
observable effect when transmitted as `Pari` and are not defined by this
specification.

## 9. Persistent Processing State (`MemP/Stat`)

### 9.1 Message form

Named processing state shall be written using:

```text
operation = SetP
object    = Appl
selector  = 0
type      = MemP
```

The defined `MemP` object has a total typed length of 2028 bytes and a data
length of 2020 bytes.

The data begins with:

```text
offset 0x00: `Stat` tag, wire bytes 74 61 74 53
offset 0x04: format version, little-endian value 2
offset 0x08: unspecified header and length fields
offset 0x15: UBJSON object
```

The `Stat` tag and format version shall have the values shown above.

Fields between offsets `0x08` and `0x14`, including two repeated length-like
values, are unspecified. The UBJSON start offset may vary by message variant
and shall not be assumed without validating the header.

### 9.2 UBJSON encoding

The state document uses UBJSON conventions:

- object keys are length-prefixed;
- string values use the `S` marker;
- small integers use integer markers such as `i`;
- floating-point values use the `d` marker and big-endian IEEE-754 binary32;
- nested processing sections are UBJSON objects.

The use of big-endian floating-point values inside UBJSON is an exception to
the general little-endian protocol convention.

### 9.3 State schema

Known top-level keys are:

```text
preset_name
opt
filter
gate
comp
eq
limit
voicefx
```

Known fields include:

```text
preset_name
opt.swapcompeq

filter.hpf

gate.on
gate.keylisten
gate.expander
gate.keyfilter
gate.threshold
gate.range
gate.attack
gate.release

comp.on
comp.softknee
comp.automode
comp.threshold
comp.ratio
comp.attack
comp.release
comp.gain
comp.keyfilter
comp.keylisten
comp.__classid

eq.eqallon
eq.eqgain1 .. eq.eqgain4
eq.eqq1 .. eq.eqq4
eq.eqfreq1 .. eq.eqfreq4
eq.eqbandon1 .. eq.eqbandon4
eq.eqbandop1
eq.eqbandop4
eq.__classid

limit.limiteron
limit.threshold

voicefx.on
```

User-facing quantities in this object are stored in natural units:

- thresholds and gains are in dB;
- frequencies are in Hz;
- attack and release values are in seconds;
- ratios are ordinary numeric ratios;
- enable fields are integer zero or one.

`MemP/Stat` shall be the named-state representation for this protocol profile.
Effect-specific real-time blocks shall contain the corresponding derived
coefficients and converted threshold values.

### 9.4 Preset application behavior

A preset application consists of effect-specific real-time writes for the target
channel followed by a `SetP/Appl/MemP` `Stat` snapshot. The snapshot's
`preset_name` identifies the applied preset. Captured built-in names include:

```text
Vocal
Acoustic Guitar
Electric Guitar
Bass Guitar
Stereo Acoustic
Stereo Piano
Stereo DJ
```

The controller does not send a dedicated preset-number parameter in the defined
capture profile. It may omit unchanged effect blocks, and different presets may
therefore produce different write sequences. Some transitions use repeated EQ
coefficient updates, including the provisional `Lfdf` and `MLff` objects, to
interpolate between states.

A host shall acknowledge each effect-specific write independently. It shall not
infer preset identity from the number or order of the preceding DSP blocks; the
`preset_name` in the final `Stat` document is the defined identity.

### 9.5 Commit and synchronization behavior

The host should write `MemP/Stat` after real-time effect changes settle to
synchronize the named processing state.

This specification does not define:

- whether every real-time update must eventually be followed by `MemP/Stat`;
- whether `MemP/Stat` alone applies DSP state;
- whether the state survives power cycling;
- whether a minimum debounce/commit interval is required;
- how the 2028-byte fixed size is padded and validated.

### 9.6 Provisional custom-preset store variant (`MemP/PrsM`)

A custom-preset store uses the same `SetP/Appl/MemP` envelope and 2028-byte
typed length as `Stat`, but its data begins with a `PrsM` tag:

```text
offset 0x00: `PrsM` tag, wire bytes 4d 73 72 50
offset 0x04: little-endian value 22
offset 0x08: unspecified header and length fields
offset 0x15: UBJSON object
```

The captured store to custom slot 1 began with these 21 data bytes:

```text
4d 73 72 50 16 00 00 00 00 00 00 00 00 00 00 0a
02 0a 02 00 00
```

Its UBJSON document used `preset_name = "custom"`. Recalling that preset used
ordinary effect-specific writes followed by a `MemP/Stat` document with the
same name; no dedicated slot-selection transaction was observed.

The slot-number encoding, the representation of the six user-visible slots,
the meaning of value 22 and the intermediate header bytes, and persistence
semantics are unspecified. An implementation shall generate `PrsM` only from a
compatible template until those fields are characterized.

## 10. Control Encoding Summary

| Control | Operation/object/type | User value encoding |
|---|---|---|
| Channel preset enable | `SetP/Appl/Para` | Parameter ID 4; `f32` 0.0 or 1.0 with zero-based channel index |
| HPF enable | `SetP/filt/MBdf` → `SetP/filt/Bqdf` | Identity or active coefficient vectors |
| HPF frequency | `SetP/filt/MBdf` → `SetP/filt/Bqdf` | Derived coefficients; named Hz value in `MemP/Stat` |
| Gate enable | `SetP/gate/gate` | Data dword 13, `u32` 0 or 1 |
| Gate threshold | `SetP/gate/gate` | `10^(-dB/20)` in data dword 7 |
| Gate key filter | `SetP/gate/MBdf` → `SetP/gate/gate` | Four sample-rate coefficient records plus gate block |
| Compressor enable | `SetP/comp/cpxt` | Data dword 12, `u32` 0 or 1 |
| Compressor threshold | `SetP/comp/cpxt` | Direct dB float in data dword 10 |
| Compressor key filter | `SetP/comp/cpxt` → `SetP/comp/MBdf` | Compressor block plus four sample-rate coefficient records |
| EQ global enable | `SetP/eq  /MBdf` → `SetP/eq  /Bqdf` (per band) | Identity or active vectors for four bands |
| EQ band update | `SetP/eq  /MBdf` → `SetP/eq  /Bqdf` | Derived coefficients for one section index |
| Limiter enable | `SetP/lim /lim ` | Data dword 1, `u32` 0 or 1 |
| Limiter threshold | `SetP/lim /lim ` | `10^(-dB/20)` in data dword 2 |
| Per-channel phantom power | `SetP/Appl/Pari` | Parameter ID 0, zero-based channel target; integer 0 or 1 |
| Channel link | `SetP/Appl/Pari` | Parameter ID 9 for both channel targets; integer 0 or 1 |
| Monitor source | `SetP/Appl/Pari` | Parameter ID 11, target 0; Main 0, Mix A 1, Mix B 2 |
| Mix Main Mono | `SetP/0x00000064/Pari` | Parameter ID 8, target 0; stereo 0, mono 1 |
| Processing order | `SetP/opt /Pari` | Integer parameter ID 0, boolean value |
| Reverb send | `SetP/0x00000000/Para` | Generic `Para`, parameter ID 1, direct dB float |
| Pan | `SetP/0x00000000/Para` | Generic `Para`, parameter ID 2, normalized float 0.0–1.0 |
| Channel gain | `SetP/Appl/Para` | Generic `Para`, parameter ID 3; direct dB float |
| Named processing state | `SetP/Appl/MemP` | UBJSON `Stat` document |

## 11. Research Gaps and Unspecified Behavior

This section lists behavior intentionally left unspecified by this version.
These items define the priorities for further protocol characterization. An
implementation shall not infer portable behavior for any listed item without
a compatible device or firmware profile.

### 11.1 Transport and transactions

- Non-zero response status codes and their meanings.
- Maximum number of simultaneous outstanding requests.
- Timeout and retry requirements.
- Device behavior for fragmented or concatenated messages outside the defined
  transport profile.
- Behavior when `reserved` is non-zero.

### 11.2 Addressing

- Whether the zero-based selector convention applies to all effect families.
- Selector meanings for output, bus, and linked-channel processing.
- Additional operation, object, and typed-object tags.

### 11.3 Application state

- Complete mapping of `JaSt` dwords 0 through 37.
- Meanings of both channel-link/activity masks.
- Meanings of state dwords 40 through 43.
- Meaning of `status_flags` bit 0 and all unassigned status bits.
- Meanings and lifetime of the final state/event bytes other than the defined
  per-channel phantom-power bits.
- Meter scaling, smoothing, clipping, and update rate.

### 11.4 DSP blocks

- Gate dword 12 and compressor dword 9 semantics.
- Gate and compressor key-filter coefficient sign/order beyond the stable
  five-float record layout.
- Biquad coefficient order and sign convention.
- Coefficient generation formulas and sample-rate dependency.
- Complete `MBdf` layout and whether it is mandatory.
- Whether `MBdf` must be sent before every corresponding `Bqdf` update or only
  when the filter family changes.
- Limiter smoothing/time-constant calculation.
- Fader parameter ID and wire representation.
- Complete `mprm` object schema and update semantics.
- Meaning of the `0x00000000` object used by generic parameters.
- Valid ranges and quantization rules for reverb send, channel gain, and pan.
- Whether the same DSP block layouts and selector convention apply to linked
  channels, buses, and outputs.
- Controls for key listen, expander mode, compressor modes, per-band EQ mode,
  voice effects, and preset selection.
- Channel hot-key control and readback. A recorded off/on/off operation produced
  no identified vendor-protocol write or stable `JaSt` transition.

### 11.5 Persistent state

- Complete `MemP/Stat` binary header.
- Meaning of the duplicated length-like fields before UBJSON.
- Padding and integrity requirements.
- Commit, load, and power-cycle persistence behavior.
- Supported UBJSON markers, limits, and unknown-key handling.
- Semantics and permitted values of class identifiers.
- Complete `MemP/PrsM` header, custom-slot encoding, and behavior for slots
  other than the captured slot 1.

## 12. Compatibility Requirements

A forward-compatible implementation should:

1. validate all outer and typed-object lengths before decoding;
2. preserve unknown fields in partially understood DSP blocks;
3. accept unknown tags as opaque values rather than terminating;
4. decode all fixed values explicitly with the correct endianness;
5. match responses by nonce and reject non-zero status;
6. preserve unmodified UBJSON keys when updating named state;
7. construct `MBdf` only from a compatible template or coefficient generator;
8. poll `JaSt` to observe physical-device preset changes;
9. retain raw messages for unsupported object and firmware variants.
