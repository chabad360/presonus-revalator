use crate::{ValueScale, control::ControlValue};
use egui::{Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use std::ops::RangeInclusive;

/// A vertical audio fader with linear, logarithmic, or audio-taper value mapping.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Fader<'a> {
    control: ControlValue<'a>,
    text: Option<String>,
    size: Vec2,
    decibels: bool,
}

impl<'a> Fader<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            control: ControlValue::new(value, range),
            text: None,
            size: Vec2::new(96.0, 220.0),
            decibels: false,
        }
    }

    pub fn scale(mut self, scale: ValueScale) -> Self {
        self.control.set_scale(scale);
        self.decibels = false;
        self
    }

    /// Treats the stored value as decibels, applies an audio fader taper, and
    /// draws common mixer-fader tick marks.
    pub fn decibels(mut self) -> Self {
        self.control.set_scale(ValueScale::AudioTaper);
        self.decibels = true;
        self.control.set_suffix(" dB".to_owned());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.control.set_prefix(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.control.set_suffix(suffix.into());
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn reset_value(mut self, value: f32) -> Self {
        self.control.set_reset_value(value);
        self
    }

    /// Sets the value-domain increment used by arrow keys.
    pub fn keyboard_step(mut self, step: f32) -> Self {
        self.control.set_keyboard_step(step);
        self
    }

    /// Sets the value-domain increment used by Shift+arrow keys.
    pub fn fine_keyboard_step(mut self, step: f32) -> Self {
        self.control.set_fine_keyboard_step(step);
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.control.set_decimals(decimals);
        self
    }

    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.control.set_formatter(formatter);
        self
    }

    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.control.set_parser(parser);
        self
    }

    pub fn notch(mut self, value: f32) -> Self {
        self.control.add_notch(value);
        self
    }

    pub fn notches(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.control.add_notches(values);
        self
    }

    pub fn notch_tolerance(mut self, tolerance: f32) -> Self {
        self.control.set_notch_tolerance(tolerance);
        self
    }
}

impl Widget for Fader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut control,
            text,
            size,
            decibels,
        } = self;
        let (rect, outer_response) = ui.allocate_exact_size(size, Sense::hover());
        let label_height = if text.is_some() { 20.0 } else { 4.0 };
        let value_height = 20.0;
        let track = Rect::from_min_max(
            Pos2::new(rect.left() + 22.0, rect.top() + value_height + 8.0),
            Pos2::new(rect.left() + 28.0, rect.bottom() - label_height),
        );
        let mut response = ui.interact(
            track.expand2(Vec2::new(18.0, 7.0)),
            outer_response.id.with("track"),
            Sense::click_and_drag(),
        );

        control.focus_on_click(&response);
        let fine = ui.input(|input| input.modifiers.shift);
        if control.reset_on_double_click(&mut response) {
            // A configured reset takes precedence over either drag mode.
        } else if fine {
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                let normalized = control.normalized_value() - delta.y / 800.0;
                control.update_normalized_value(normalized, &mut response);
            }
        } else if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let normalized = egui::remap_clamp(pointer.y, track.bottom()..=track.top(), 0.0..=1.0);
            control.update_normalized_value(normalized, &mut response);
        }

        let hovered = ui.rect_contains_pointer(rect);
        control.handle_hover_input(ui, &mut response, hovered);

        let visuals = *ui.style().interact(&response);
        let painter = ui.painter_at(rect);
        painter.rect_filled(track, 3.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(track, 3.0, visuals.bg_stroke, egui::StrokeKind::Inside);

        let normalized = control.normalized_value();
        let thumb_y = egui::lerp(track.bottom()..=track.top(), normalized);
        let filled = Rect::from_min_max(
            Pos2::new(track.left(), thumb_y),
            Pos2::new(track.right(), track.bottom()),
        );
        painter.rect_filled(filled, 3.0, ui.visuals().selection.bg_fill);

        if decibels {
            paint_decibel_ticks(ui, &painter, track, control.scale(), control.range());
        } else {
            paint_general_ticks(ui, &painter, track, control.scale(), control.range());
        }
        paint_notches(
            &painter,
            track,
            control.scale(),
            control.range(),
            control.notches(),
            visuals.fg_stroke.color,
        );

        let thumb =
            Rect::from_center_size(Pos2::new(track.center().x, thumb_y), Vec2::new(28.0, 18.0));
        painter.rect_filled(thumb, 3.0, visuals.bg_fill);
        painter.rect_stroke(thumb, 3.0, visuals.bg_stroke, egui::StrokeKind::Inside);
        painter.line_segment(
            [
                Pos2::new(thumb.left() + 4.0, thumb.center().y),
                Pos2::new(thumb.right() - 4.0, thumb.center().y),
            ],
            Stroke::new(2.0_f32, visuals.fg_stroke.color),
        );

        if response.has_focus() {
            painter.rect_stroke(
                thumb.expand(3.0),
                4.0,
                Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Outside,
            );
        }

        let value_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + value_height * 0.5),
            Vec2::new(76.0, value_height),
        );
        let value_response = control.add_editable(
            ui,
            outer_response.id.with("value"),
            value_rect,
            &mut response,
        );

        if let Some(text) = text {
            painter.text(
                Pos2::new(rect.center().x, rect.bottom()),
                egui::Align2::CENTER_BOTTOM,
                text,
                FontId::proportional(12.0),
                visuals.text_color(),
            );
        }

        if response.hovered() {
            response = response.on_hover_text(
                "Drag the fader, or hover and use the wheel or arrow keys. Hold Shift for fine control; click the value to type.",
            );
        }
        outer_response.union(response).union(value_response)
    }
}

fn paint_decibel_ticks(
    ui: &Ui,
    painter: &egui::Painter,
    track: Rect,
    scale: ValueScale,
    range: &RangeInclusive<f32>,
) {
    const TICKS: &[f32] = &[
        -60.0, -50.0, -40.0, -30.0, -20.0, -10.0, -5.0, 0.0, 5.0, 10.0,
    ];
    let text_color = ui.visuals().weak_text_color();
    let minimum = *range.start();
    let maximum = *range.end();

    if !TICKS.contains(&minimum) {
        paint_tick(painter, track, 0.0, format!("{minimum:.0}"), text_color);
    }
    if !TICKS.contains(&maximum) {
        paint_tick(painter, track, 1.0, format!("{maximum:.0}"), text_color);
    }

    for &tick in TICKS {
        if range.contains(&tick) {
            paint_tick(
                painter,
                track,
                scale.normalize(tick, range),
                format!("{tick:.0}"),
                text_color,
            );
        }
    }
}

fn paint_general_ticks(
    ui: &Ui,
    painter: &egui::Painter,
    track: Rect,
    scale: ValueScale,
    range: &RangeInclusive<f32>,
) {
    let text_color = ui.visuals().weak_text_color();
    for normalized in [0.0, 0.5, 1.0] {
        let value = scale.denormalize(normalized, range);
        paint_tick(
            painter,
            track,
            normalized,
            format!("{value:.1}"),
            text_color,
        );
    }
}

fn paint_notches(
    painter: &egui::Painter,
    track: Rect,
    scale: ValueScale,
    range: &RangeInclusive<f32>,
    notches: &[f32],
    color: Color32,
) {
    for notch in notches
        .iter()
        .copied()
        .filter(|value| range.contains(value))
    {
        let y = egui::lerp(track.bottom()..=track.top(), scale.normalize(notch, range));
        painter.line_segment(
            [
                Pos2::new(track.left() - 9.0, y),
                Pos2::new(track.left() - 2.0, y),
            ],
            Stroke::new(2.0, color),
        );
    }
}

fn paint_tick(
    painter: &egui::Painter,
    track: Rect,
    normalized: f32,
    label: String,
    color: Color32,
) {
    let y = egui::lerp(track.bottom()..=track.top(), normalized);
    painter.line_segment(
        [
            Pos2::new(track.right() + 2.0, y),
            Pos2::new(track.right() + 7.0, y),
        ],
        Stroke::new(1.0_f32, color),
    );
    painter.text(
        Pos2::new(track.right() + 10.0, y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::monospace(9.0),
        color,
    );
}
