use crate::amplitude_to_db;
use egui::{Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use std::ops::RangeInclusive;

const CHANNEL_WIDTH: f32 = 10.0;
const CHANNEL_GAP: f32 = 2.0;

/// Values accepted by [`LevelMeter::new`] and [`LevelMeter::from_db`].
pub trait IntoMeterValues {
    fn into_meter_values(self) -> Vec<f32>;
}

impl IntoMeterValues for f32 {
    fn into_meter_values(self) -> Vec<f32> {
        vec![self]
    }
}

impl IntoMeterValues for Vec<f32> {
    fn into_meter_values(self) -> Vec<f32> {
        self
    }
}

impl IntoMeterValues for &[f32] {
    fn into_meter_values(self) -> Vec<f32> {
        self.to_vec()
    }
}

impl<const N: usize> IntoMeterValues for [f32; N] {
    fn into_meter_values(self) -> Vec<f32> {
        self.to_vec()
    }
}

/// Colors used by a [`LevelMeter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeterColors {
    pub normal: Color32,
    pub warning: Color32,
    pub clip: Color32,
    pub peak: Color32,
    pub background: Color32,
}

impl Default for MeterColors {
    fn default() -> Self {
        Self {
            normal: Color32::from_rgb(46, 204, 113),
            warning: Color32::from_rgb(241, 196, 15),
            clip: Color32::from_rgb(231, 76, 60),
            peak: Color32::WHITE,
            background: Color32::from_rgb(25, 29, 32),
        }
    }
}

/// A colored single- or multi-channel meter with an IEC-style nonlinear dB scale.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct LevelMeter {
    values: Vec<f32>,
    values_are_db: bool,
    range_db: RangeInclusive<f32>,
    warning_db: f32,
    clip_db: f32,
    inverted: bool,
    clipping: Vec<Option<bool>>,
    channel_names: Vec<Option<String>>,
    text: Option<String>,
    size: Vec2,
    colors: MeterColors,
    peak_hold_seconds: f64,
    peak_decay_per_second: f32,
}

impl LevelMeter {
    /// Creates a meter from one or more linear amplitude ratios, where `1.0` is 0 dBFS.
    ///
    /// A scalar creates a single-channel meter. Arrays, slices, and vectors create
    /// a compact multi-channel meter with adjacent bars.
    pub fn new(values: impl IntoMeterValues) -> Self {
        Self::with_values(values.into_meter_values(), false)
    }

    /// Creates a meter from one or more values already expressed in decibels.
    pub fn from_db(values: impl IntoMeterValues) -> Self {
        Self::with_values(values.into_meter_values(), true)
    }

    fn with_values(values: Vec<f32>, values_are_db: bool) -> Self {
        assert!(!values.is_empty(), "a meter requires at least one channel");
        let channel_count = values.len();
        let channel_width = channel_count as f32 * (CHANNEL_WIDTH + CHANNEL_GAP) - CHANNEL_GAP;
        Self {
            values,
            values_are_db,
            range_db: -60.0..=6.0,
            warning_db: -12.0,
            clip_db: 0.0,
            inverted: false,
            clipping: vec![None; channel_count],
            channel_names: vec![None; channel_count],
            text: None,
            size: Vec2::new((channel_width + 46.0).max(72.0), 220.0),
            colors: MeterColors::default(),
            peak_hold_seconds: 0.75,
            peak_decay_per_second: 0.35,
        }
    }

    /// Fills downward from the maximum value as the input decreases.
    pub fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    /// Attaches one clipping state to all channels.
    ///
    /// A meter without this call draws no clipping indicators.
    pub fn clipping(mut self, clipping: bool) -> Self {
        self.clipping = vec![Some(clipping); self.values.len()];
        self
    }

    /// Attaches a separate clipping state to each channel.
    pub fn channel_clipping(mut self, clipping: impl IntoIterator<Item = bool>) -> Self {
        let clipping = clipping.into_iter().map(Some).collect::<Vec<_>>();
        assert_eq!(
            clipping.len(),
            self.values.len(),
            "clipping states must match the meter channel count"
        );
        self.clipping = clipping;
        self
    }

    /// Attaches optional clipping states channel-by-channel.
    pub fn optional_channel_clipping(
        mut self,
        clipping: impl IntoIterator<Item = Option<bool>>,
    ) -> Self {
        let clipping = clipping.into_iter().collect::<Vec<_>>();
        assert_eq!(
            clipping.len(),
            self.values.len(),
            "clipping states must match the meter channel count"
        );
        self.clipping = clipping;
        self
    }

    /// Assigns optional names used in the per-channel tooltip.
    pub fn optional_channel_names<S>(mut self, names: impl IntoIterator<Item = Option<S>>) -> Self
    where
        S: Into<String>,
    {
        let names = names
            .into_iter()
            .map(|name| name.map(Into::into))
            .collect::<Vec<_>>();
        assert_eq!(
            names.len(),
            self.values.len(),
            "channel names must match the meter channel count"
        );
        self.channel_names = names;
        self
    }

    pub fn range_db(mut self, range: RangeInclusive<f32>) -> Self {
        self.range_db = range;
        self
    }

    pub fn warning_db(mut self, warning_db: f32) -> Self {
        self.warning_db = warning_db;
        self
    }

    pub fn clip_db(mut self, clip_db: f32) -> Self {
        self.clip_db = clip_db;
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn colors(mut self, colors: MeterColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn peak_hold_seconds(mut self, seconds: f64) -> Self {
        self.peak_hold_seconds = seconds.max(0.0);
        self
    }

    pub fn peak_decay_per_second(mut self, amount: f32) -> Self {
        self.peak_decay_per_second = amount.max(0.0);
        self
    }
}

impl Widget for LevelMeter {
    fn ui(self, ui: &mut Ui) -> Response {
        let (minimum_db, maximum_db) = (*self.range_db.start(), *self.range_db.end());
        assert!(
            minimum_db.is_finite() && maximum_db.is_finite() && minimum_db < maximum_db,
            "meter dB range must contain two increasing finite values"
        );
        assert!(
            self.warning_db >= minimum_db
                && self.warning_db <= self.clip_db
                && self.clip_db <= maximum_db,
            "meter thresholds must be ordered and inside its dB range"
        );

        let (rect, mut response) = ui.allocate_exact_size(self.size, Sense::hover());
        let painter = ui.painter_at(rect);
        let visuals = ui.style().interact(&response);
        let label_height = if self.text.is_some() { 20.0 } else { 4.0 };
        let show_value = self.values.len() == 1;
        let top_padding = if show_value { 34.0 } else { 14.0 };
        let meter_width = self.values.len() as f32 * (CHANNEL_WIDTH + CHANNEL_GAP) - CHANNEL_GAP;
        let meter_box = Rect::from_min_size(
            Pos2::new(rect.left() + 8.0, rect.top() + top_padding),
            Vec2::new(meter_width, rect.height() - top_padding - label_height),
        );
        let now = ui.input(|input| input.time);
        let mut tooltip_lines = Vec::with_capacity(self.values.len());
        let mut repaint_for_peak = false;

        painter.rect_filled(meter_box, 2.0, self.colors.background);
        for (index, &value) in self.values.iter().enumerate() {
            let channel = channel_rect(meter_box, index);
            let db = if self.values_are_db {
                value
            } else {
                amplitude_to_db(value.abs())
            };
            let shown_db = if db.is_finite() {
                db.clamp(minimum_db, maximum_db)
            } else {
                minimum_db
            };
            let position = meter_position(db, &self.range_db);
            let activity = if self.inverted {
                1.0 - position
            } else {
                position
            };

            if self.inverted {
                let fill = Rect::from_min_max(
                    channel.left_top(),
                    Pos2::new(
                        channel.right(),
                        egui::lerp(channel.top()..=channel.bottom(), activity),
                    ),
                );
                painter.rect_filled(fill, 0.0, self.colors.normal);
            } else {
                paint_zone(
                    &painter,
                    channel,
                    &self.range_db,
                    minimum_db,
                    shown_db.min(self.warning_db),
                    self.colors.normal,
                );
                paint_zone(
                    &painter,
                    channel,
                    &self.range_db,
                    self.warning_db,
                    shown_db.min(self.clip_db),
                    self.colors.warning,
                );
                paint_zone(
                    &painter,
                    channel,
                    &self.range_db,
                    self.clip_db,
                    shown_db,
                    self.colors.clip,
                );
            }

            let state_id = response.id.with(("peak", index));
            let mut peak_state = ui
                .data_mut(|data| data.get_temp::<PeakState>(state_id))
                .unwrap_or_else(|| PeakState::new(activity, now, self.peak_hold_seconds));
            update_peak(
                &mut peak_state,
                activity,
                now,
                self.peak_hold_seconds,
                self.peak_decay_per_second,
            );
            ui.data_mut(|data| data.insert_temp(state_id, peak_state));
            let peak_y = if self.inverted {
                egui::lerp(channel.top()..=channel.bottom(), peak_state.activity)
            } else {
                egui::lerp(channel.bottom()..=channel.top(), peak_state.activity)
            };
            painter.line_segment(
                [
                    Pos2::new(channel.left(), peak_y),
                    Pos2::new(channel.right(), peak_y),
                ],
                Stroke::new(2.0, self.colors.peak),
            );
            repaint_for_peak |= peak_state.activity > activity + f32::EPSILON;

            if let Some(clipping) = self.clipping[index] {
                let indicator = Rect::from_min_max(
                    Pos2::new(channel.left(), meter_box.top() - 7.0),
                    Pos2::new(channel.right(), meter_box.top() - 3.0),
                );
                painter.rect_filled(
                    indicator,
                    0.0,
                    if clipping {
                        self.colors.clip
                    } else {
                        self.colors.background
                    },
                );
                painter.rect_stroke(
                    indicator,
                    0.0,
                    Stroke::new(1.0, self.colors.clip),
                    egui::StrokeKind::Inside,
                );
            }

            let unit = if self.values_are_db { "dB" } else { "dBFS" };
            let value_text = if db.is_finite() {
                format!("{db:.1} {unit}")
            } else {
                format!("−∞ {unit}")
            };
            let channel_name = self.channel_names[index]
                .as_deref()
                .map_or_else(|| format!("Channel {}", index + 1), str::to_owned);
            tooltip_lines.push(format!("{channel_name}: {value_text}"));
            if show_value {
                painter.text(
                    Pos2::new(rect.left(), rect.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    &value_text,
                    FontId::proportional(11.0),
                    visuals.text_color(),
                );
            }
        }

        paint_ticks(ui, &painter, meter_box, &self.range_db, self.values.len());
        painter.rect_stroke(
            meter_box,
            2.0,
            Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
            egui::StrokeKind::Inside,
        );
        if repaint_for_peak {
            ui.ctx().request_repaint();
        }

        if let Some(text) = self.text {
            painter.text(
                Pos2::new(rect.center().x, rect.bottom()),
                egui::Align2::CENTER_BOTTOM,
                text,
                FontId::proportional(12.0),
                visuals.text_color(),
            );
        }

        if response.hovered() {
            response = response.on_hover_text(tooltip_lines.join("\n"));
        }
        response
    }
}

fn channel_rect(meter_box: Rect, index: usize) -> Rect {
    let left = meter_box.left() + index as f32 * (CHANNEL_WIDTH + CHANNEL_GAP);
    Rect::from_min_max(
        Pos2::new(left, meter_box.top()),
        Pos2::new(left + CHANNEL_WIDTH, meter_box.bottom()),
    )
}

#[derive(Debug, Clone, Copy)]
struct PeakState {
    activity: f32,
    last_update: f64,
    hold_until: f64,
}

impl PeakState {
    fn new(activity: f32, now: f64, hold_seconds: f64) -> Self {
        Self {
            activity,
            last_update: now,
            hold_until: now + hold_seconds,
        }
    }
}

fn update_peak(
    state: &mut PeakState,
    activity: f32,
    now: f64,
    hold_seconds: f64,
    decay_per_second: f32,
) {
    let elapsed = (now - state.last_update).max(0.0) as f32;
    if activity >= state.activity {
        state.activity = activity;
        state.hold_until = now + hold_seconds;
    } else if now >= state.hold_until {
        state.activity = (state.activity - elapsed * decay_per_second).max(activity);
    }
    state.last_update = now;
}

fn paint_zone(
    painter: &egui::Painter,
    track: Rect,
    range: &RangeInclusive<f32>,
    start_db: f32,
    end_db: f32,
    color: Color32,
) {
    if end_db <= start_db {
        return;
    }

    let start = meter_position(start_db, range);
    let end = meter_position(end_db, range);
    let zone = Rect::from_min_max(
        Pos2::new(track.left(), egui::lerp(track.bottom()..=track.top(), end)),
        Pos2::new(
            track.right(),
            egui::lerp(track.bottom()..=track.top(), start),
        ),
    );
    painter.rect_filled(zone, 0.0, color);
}

fn paint_ticks(
    ui: &Ui,
    painter: &egui::Painter,
    meter_box: Rect,
    range: &RangeInclusive<f32>,
    channels: usize,
) {
    const TICKS: &[f32] = &[
        -60.0, -50.0, -40.0, -30.0, -20.0, -10.0, -5.0, 0.0, 3.0, 6.0,
    ];
    let color = ui.visuals().weak_text_color();

    for &tick in TICKS {
        if !range.contains(&tick) {
            continue;
        }
        let y = egui::lerp(
            meter_box.bottom()..=meter_box.top(),
            meter_position(tick, range),
        );
        for index in 0..channels {
            let channel = channel_rect(meter_box, index);
            painter.line_segment(
                [
                    Pos2::new(channel.left() + 1.0, y),
                    Pos2::new(channel.left() + 3.5, y),
                ],
                Stroke::new(1.0_f32, color),
            );
            painter.line_segment(
                [
                    Pos2::new(channel.right() - 3.5, y),
                    Pos2::new(channel.right() - 1.0, y),
                ],
                Stroke::new(1.0_f32, color),
            );
        }
        painter.text(
            Pos2::new(meter_box.right() + 7.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{tick:.0}"),
            FontId::monospace(9.0),
            color,
        );
    }
}

/// IEC 60268-18 style meter deflection, normalized to the requested range.
fn meter_position(db: f32, range: &RangeInclusive<f32>) -> f32 {
    if db == f32::NEG_INFINITY || db.is_nan() {
        return 0.0;
    }
    if db == f32::INFINITY {
        return 1.0;
    }

    let db = db.clamp(*range.start(), *range.end());
    (iec_deflection(db) / iec_deflection(*range.end())).clamp(0.0, 1.0)
}

fn iec_deflection(db: f32) -> f32 {
    if db < -60.0 {
        egui::remap_clamp(db, -120.0..=-60.0, 0.0..=0.025)
    } else if db < -50.0 {
        0.025 + (db + 60.0) * 0.005
    } else if db < -40.0 {
        0.075 + (db + 50.0) * 0.0075
    } else if db < -30.0 {
        0.15 + (db + 40.0) * 0.015
    } else if db < -20.0 {
        0.30 + (db + 30.0) * 0.02
    } else if db < -9.0 {
        0.50 + (db + 20.0) * (0.25 / 11.0)
    } else {
        0.75 + (db + 9.0) * (0.25 / 9.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_infinity_has_zero_deflection_but_finite_floor_does_not() {
        let range = -60.0..=0.0;

        assert_eq!(meter_position(f32::NEG_INFINITY, &range), 0.0);
        assert_eq!(meter_position(f32::NAN, &range), 0.0);
        assert_eq!(meter_position(-60.0, &range), 0.025);
        assert_eq!(meter_position(f32::INFINITY, &range), 1.0);
    }

    #[test]
    fn accepts_single_and_multiple_channels() {
        assert_eq!(LevelMeter::new(0.5).values, vec![0.5]);
        assert_eq!(LevelMeter::new([0.25, 0.5]).values, vec![0.25, 0.5]);
        assert_eq!(LevelMeter::new(vec![0.1, 0.2, 0.3]).values.len(), 3);

        let direct_db = LevelMeter::from_db([f32::NEG_INFINITY, -12.0]);
        assert!(direct_db.values_are_db);
        assert_eq!(direct_db.values, vec![f32::NEG_INFINITY, -12.0]);
    }

    #[test]
    fn supports_optional_clipping_per_channel() {
        let meter = LevelMeter::new([0.25, 0.5]).optional_channel_clipping([Some(false), None]);

        assert_eq!(meter.clipping, vec![Some(false), None]);
    }

    #[test]
    #[should_panic(expected = "clipping states must match")]
    fn requires_one_clipping_state_per_channel() {
        let _ = LevelMeter::new([0.25, 0.5]).channel_clipping([false]);
    }

    #[test]
    fn meter_uses_iec_style_breakpoints() {
        let range = -60.0..=0.0;

        assert_eq!(meter_position(-60.0, &range), 0.025);
        assert_eq!(meter_position(-20.0, &range), 0.5);
        assert_eq!(meter_position(-9.0, &range), 0.75);
        assert_eq!(meter_position(0.0, &range), 1.0);
    }

    #[test]
    fn inverted_meter_is_empty_at_zero_and_fills_as_reduction_increases() {
        let range = -60.0..=0.0;
        let fill = |db| 1.0 - meter_position(db, &range);

        assert_eq!(fill(0.0), 0.0);
        assert_eq!(fill(-20.0), 0.5);
        assert!((fill(-60.0) - 0.975).abs() < f32::EPSILON);
    }

    #[test]
    fn peak_holds_then_decays_slowly() {
        let mut state = PeakState::new(0.8, 0.0, 0.75);

        update_peak(&mut state, 0.2, 0.5, 0.75, 0.25);
        assert_eq!(state.activity, 0.8);
        update_peak(&mut state, 0.2, 1.0, 0.75, 0.25);
        assert_eq!(state.activity, 0.675);
        update_peak(&mut state, 0.9, 1.1, 0.75, 0.25);
        assert_eq!(state.activity, 0.9);
        assert_eq!(state.hold_until, 1.85);
    }

    #[test]
    fn meter_position_uses_decibels_not_linear_amplitude() {
        let range = -60.0..=0.0;
        let half_amplitude_db = amplitude_to_db(0.5);
        let position = meter_position(half_amplitude_db, &range);

        assert!((half_amplitude_db - -6.0206).abs() < 0.0001);
        assert!(position > 0.8);
    }
}
