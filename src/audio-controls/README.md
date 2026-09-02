# egui-audio-controls

Audio-oriented widgets for `egui`:

- `LevelMeter` consumes a **linear amplitude** (`1.0 == 0 dBFS`), converts it to an IEC-style nonlinear meter scale, paints configurable green/yellow/red regions, accepts an external clipping signal, and holds peaks with a slow decay.
- `Knob` supports dragging, keyboard adjustment, direct numeric entry, fine adjustment with Shift, reset on double-click, magnetic notches, and linear or logarithmic value mapping.
- `Fader` provides the same input and formatting options in a vertical control with an audio taper and standard mixer markings.

## Scale semantics

Use `ValueScale::Logarithmic` when the stored value is a positive linear quantity such as frequency:

```rust
# use egui_audio_controls::{Knob, ValueScale};
# fn ui(ui: &mut egui::Ui, frequency: &mut f32) {
ui.add(
    Knob::new(frequency, 20.0..=20_000.0)
        .scale(ValueScale::Logarithmic)
        .suffix(" Hz")
        .text("Frequency"),
);
# }
```

The value shown above a knob or fader is editable. Prefixes, suffixes, custom formatting/parsing, and magnetic notches can be combined without assigning a unit-specific widget mode:

```rust
# use egui_audio_controls::Knob;
# fn ui(ui: &mut egui::Ui, pan: &mut f32, volume: &mut f32) {
ui.add(Knob::new(pan, 0.0..=1.0).notch(0.5).text("Pan"));
ui.add(
    Knob::new(volume, 0.0..=1.0)
        .suffix("%")
        .custom_formatter(|value, _| format!("{:.0}", value * 100.0))
        .custom_parser(|text| text.parse::<f64>().ok().map(|value| value / 100.0))
        .text("Volume"),
);
# }
```

Use `.decibels()` when a fader's stored value is already in dB. It applies an audio taper that compresses deep attenuation near the bottom of the track and leaves more travel around unity gain:

```rust
# use egui_audio_controls::Fader;
# fn ui(ui: &mut egui::Ui, gain_db: &mut f32) {
ui.add(
    Fader::new(gain_db, -96.0..=10.0)
        .decibels()
        .reset_value(0.0)
        .text("Gain"),
);
# }
```

Meters deliberately have a separate API because their input is linear amplitude while their display position is logarithmic. Pass a scalar for one channel or an array, slice, or vector for a compact multi-channel meter:

```rust
# use egui_audio_controls::LevelMeter;
# fn stereo(ui: &mut egui::Ui, left: f32, right: f32) {
ui.add(LevelMeter::new([left, right]).text("L / R"));
# }
```

```rust
# use egui_audio_controls::LevelMeter;
# fn ui(ui: &mut egui::Ui, peak_amplitude: f32, clipping: bool) {
ui.add(
    LevelMeter::new(peak_amplitude)
        .range_db(-60.0..=6.0)
        .warning_db(-12.0)
        .clip_db(0.0)
        .clipping(clipping)
        .text("Input"),
);
# }
```

For gain reduction that rests at `0 dB` and becomes negative when processing is active, use a direct dB input and invert the meter:

```rust
# use egui_audio_controls::LevelMeter;
# fn ui(ui: &mut egui::Ui, reduction_db: f32) {
ui.add(
    LevelMeter::from_db(reduction_db)
        .range_db(-60.0..=0.0)
        .inverted()
        .text("Compression"),
);
# }
```
