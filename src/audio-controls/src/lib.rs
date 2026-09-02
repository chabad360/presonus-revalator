//! Audio-oriented controls for [`egui`].
//!
//! This crate keeps the distinction between linear amplitude and decibels explicit:
//!
//! - [`LevelMeter`] accepts one or arbitrarily many adjacent channels; linear
//!   amplitudes use [`LevelMeter::new`], while [`LevelMeter::from_db`] supports
//!   direct and inverted gain-reduction meters.
//! - [`Knob`] and [`Fader`] can map positive values logarithmically with
//!   [`ValueScale::Logarithmic`].
//! - [`Fader::decibels`] applies an audio taper that gives deep attenuation
//!   little travel and leaves more room around unity gain.
//! - Knobs and faders provide editable values, prefixes/suffixes, custom
//!   formatters/parsers, and magnetic notches.
//!
//! ```
//! # use egui_audio_controls::{Fader, Knob, LevelMeter, ValueScale};
//! # fn controls(ui: &mut egui::Ui, gain_db: &mut f32, frequency: &mut f32, peak: f32) {
//! ui.add(LevelMeter::new(peak).text("Input"));
//! ui.add(Fader::new(gain_db, -96.0..=10.0).decibels().text("Gain"));
//! ui.add(
//!     Knob::new(frequency, 20.0..=20_000.0)
//!         .scale(ValueScale::Logarithmic)
//!         .suffix(" Hz")
//!         .text("Frequency"),
//! );
//! # }
//! ```

mod control;
mod fader;
mod knob;
mod meter;
mod scale;

pub use fader::Fader;
pub use knob::Knob;
pub use meter::{IntoMeterValues, LevelMeter, MeterColors};
pub(crate) use scale::snap_to_notch;
pub use scale::{ValueScale, amplitude_to_db, db_to_amplitude};
