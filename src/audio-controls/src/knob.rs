use crate::{ValueScale, snap_to_notch};
use egui::{DragValue, FontId, Key, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use std::f32::consts::TAU;
use std::ops::RangeInclusive;

/// A rotary control with optional logarithmic value mapping.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Knob<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    scale: ValueScale,
    text: Option<String>,
    prefix: String,
    suffix: String,
    size: Vec2,
    reset_value: Option<f32>,
    decimals: usize,
    formatter: Option<Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>>,
    parser: Option<Box<dyn 'a + Fn(&str) -> Option<f64>>>,
    notches: Vec<f32>,
    notch_tolerance: f32,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            scale: ValueScale::Linear,
            text: None,
            prefix: String::new(),
            suffix: String::new(),
            size: Vec2::new(76.0, 98.0),
            reset_value: None,
            decimals: 1,
            formatter: None,
            parser: None,
            notches: Vec::new(),
            notch_tolerance: 0.025,
        }
    }

    pub fn scale(mut self, scale: ValueScale) -> Self {
        self.scale = scale;
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn reset_value(mut self, value: f32) -> Self {
        self.reset_value = Some(value);
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Sets the function used by the editable value display.
    ///
    /// The callback has the same arguments as [`DragValue::custom_formatter`].
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.formatter = Some(Box::new(formatter));
        self
    }

    /// Sets the parser used when a value is entered directly.
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    /// Adds a visible magnetic notch at `value`.
    pub fn notch(mut self, value: f32) -> Self {
        self.notches.push(value);
        self
    }

    /// Adds visible magnetic notches at each supplied value.
    pub fn notches(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.notches.extend(values);
        self
    }

    /// Sets the magnetic notch radius as a fraction of the control's travel.
    pub fn notch_tolerance(mut self, tolerance: f32) -> Self {
        self.notch_tolerance = tolerance.max(0.0);
        self
    }
}

impl Widget for Knob<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            value,
            range,
            scale,
            text,
            prefix,
            suffix,
            size,
            reset_value,
            decimals,
            formatter,
            parser,
            notches,
            notch_tolerance,
        } = self;
        let (rect, outer_response) = ui.allocate_exact_size(size, Sense::hover());
        let label_height = if text.is_some() { 18.0 } else { 0.0 };
        let value_height = 20.0;
        let diameter = (rect
            .width()
            .min(rect.height() - label_height - value_height)
            - 8.0)
            .max(12.0);
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 4.0);
        let radius = diameter * 0.5;
        let dial_rect = Rect::from_center_size(center, Vec2::splat(diameter + 12.0));
        let mut response = ui.interact(
            dial_rect,
            outer_response.id.with("dial"),
            Sense::click_and_drag(),
        );

        if response.clicked() {
            response.request_focus();
        }

        let fine = ui.input(|input| input.modifiers.shift);
        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            let sensitivity = if fine { 800.0 } else { 200.0 };
            let normalized = scale.normalize(*value, &range) + (delta.x - delta.y) / sensitivity;
            let new_value = snap_to_notch(
                scale.denormalize(normalized, &range),
                scale,
                &range,
                &notches,
                notch_tolerance,
            );
            if new_value != *value {
                *value = new_value;
                response.mark_changed();
            }
        }

        if response.double_clicked()
            && let Some(reset_value) = reset_value
        {
            let reset_value = reset_value.clamp(*range.start(), *range.end());
            if reset_value != *value {
                *value = reset_value;
                response.mark_changed();
            }
        }

        if response.has_focus() {
            let direction = ui.input(|input| {
                i8::from(input.key_pressed(Key::ArrowUp) || input.key_pressed(Key::ArrowRight))
                    - i8::from(
                        input.key_pressed(Key::ArrowDown) || input.key_pressed(Key::ArrowLeft),
                    )
            });
            if direction != 0 {
                let step = if fine { 0.001 } else { 0.01 };
                let normalized = scale.normalize(*value, &range) + f32::from(direction) * step;
                *value = snap_to_notch(
                    scale.denormalize(normalized, &range),
                    scale,
                    &range,
                    &notches,
                    notch_tolerance,
                );
                response.mark_changed();
            }
        }

        let visuals = ui.style().interact(&response).clone();
        let painter = ui.painter_at(rect);
        let start_angle = TAU * 3.0 / 8.0;
        let sweep = TAU * 3.0 / 4.0;
        let normalized = scale.normalize(*value, &range);
        let value_angle = start_angle + normalized * sweep;

        painter.circle_filled(center, radius, visuals.bg_fill);
        painter.circle_stroke(center, radius, visuals.bg_stroke);
        painter.add(Shape::line(
            arc_points(center, radius + 3.0, start_angle, start_angle + sweep),
            Stroke::new(2.0_f32, ui.visuals().widgets.inactive.bg_stroke.color),
        ));
        painter.add(Shape::line(
            arc_points(center, radius + 3.0, start_angle, value_angle),
            Stroke::new(3.0_f32, visuals.fg_stroke.color),
        ));

        for notch in notches
            .iter()
            .copied()
            .filter(|value| range.contains(value))
        {
            let angle = start_angle + scale.normalize(notch, &range) * sweep;
            painter.line_segment(
                [
                    center + Vec2::angled(angle) * (radius + 1.0),
                    center + Vec2::angled(angle) * (radius + 6.0),
                ],
                Stroke::new(1.5, visuals.fg_stroke.color),
            );
        }

        let pointer_end = center + Vec2::angled(value_angle) * (radius * 0.72);
        painter.line_segment(
            [center, pointer_end],
            Stroke::new(2.5_f32, visuals.fg_stroke.color),
        );
        painter.circle_filled(center, 2.5, visuals.fg_stroke.color);

        if response.has_focus() {
            painter.circle_stroke(
                center,
                radius + 6.0,
                Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
            );
        }

        let value_rect = Rect::from_min_max(
            Pos2::new(rect.left(), center.y + radius + 4.0),
            Pos2::new(rect.right(), center.y + radius + 4.0 + value_height),
        );
        let mut value_widget = DragValue::new(value)
            .range(range.clone())
            .fixed_decimals(decimals)
            .prefix(prefix)
            .suffix(suffix);
        if let Some(formatter) = formatter {
            value_widget = value_widget.custom_formatter(formatter);
        }
        if let Some(parser) = parser {
            value_widget = value_widget.custom_parser(parser);
        }
        let mut value_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(outer_response.id.with("value"))
                .max_rect(value_rect)
                .layout(egui::Layout::centered_and_justified(
                    egui::Direction::LeftToRight,
                )),
        );
        let value_response = value_ui.add_sized(value_rect.size(), value_widget);
        if value_response.changed() {
            response.mark_changed();
        }

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
            response = response
                .on_hover_text("Drag vertically or horizontally. Hold Shift for fine control.");
        }
        outer_response.union(response).union(value_response)
    }
}

fn arc_points(center: Pos2, radius: f32, start: f32, end: f32) -> Vec<Pos2> {
    const SEGMENTS: usize = 48;
    (0..=SEGMENTS)
        .map(|index| {
            let fraction = index as f32 / SEGMENTS as f32;
            center + Vec2::angled(egui::lerp(start..=end, fraction)) * radius
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_to_nearby_notch_but_not_distant_one() {
        let range = 0.0..=1.0;
        let notches = [0.5];

        assert_eq!(
            snap_to_notch(0.51, ValueScale::Linear, &range, &notches, 0.025),
            0.5
        );
        assert_eq!(
            snap_to_notch(0.6, ValueScale::Linear, &range, &notches, 0.025),
            0.6
        );
    }
}
