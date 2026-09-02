use crate::{ValueScale, control::ControlValue};
use egui::{FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use std::f32::consts::TAU;
use std::ops::RangeInclusive;

/// A rotary control with optional logarithmic value mapping.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Knob<'a> {
    control: ControlValue<'a>,
    text: Option<String>,
    size: Vec2,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            control: ControlValue::new(value, range),
            text: None,
            size: Vec2::new(76.0, 98.0),
        }
    }

    pub fn scale(mut self, scale: ValueScale) -> Self {
        self.control.set_scale(scale);
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

    /// Sets the function used by the editable value display.
    ///
    /// The callback has the same arguments as [`egui::DragValue::custom_formatter`].
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.control.set_formatter(formatter);
        self
    }

    /// Sets the parser used when a value is entered directly.
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.control.set_parser(parser);
        self
    }

    /// Adds a visible magnetic notch at `value`.
    pub fn notch(mut self, value: f32) -> Self {
        self.control.add_notch(value);
        self
    }

    /// Adds visible magnetic notches at each supplied value.
    pub fn notches(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.control.add_notches(values);
        self
    }

    /// Sets the magnetic notch radius as a fraction of the control's travel.
    pub fn notch_tolerance(mut self, tolerance: f32) -> Self {
        self.control.set_notch_tolerance(tolerance);
        self
    }
}

impl Widget for Knob<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut control,
            text,
            size,
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

        control.focus_on_click(&response);

        let fine = ui.input(|input| input.modifiers.shift);
        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            let sensitivity = if fine { 800.0 } else { 200.0 };
            let normalized = control.normalized_value() + (delta.x - delta.y) / sensitivity;
            control.update_normalized_value(normalized, &mut response);
        }

        control.reset_on_double_click(&mut response);
        let hovered = ui.rect_contains_pointer(rect);
        control.handle_hover_input(ui, &mut response, hovered);

        let visuals = *ui.style().interact(&response);
        let painter = ui.painter_at(rect);
        let start_angle = TAU * 3.0 / 8.0;
        let sweep = TAU * 3.0 / 4.0;
        let normalized = control.normalized_value();
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

        for notch in control
            .notches()
            .iter()
            .copied()
            .filter(|value| control.range().contains(value))
        {
            let angle = start_angle + control.scale().normalize(notch, control.range()) * sweep;
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
            response = response
                .on_hover_text("Drag vertically or horizontally. Hover and use the wheel or arrow keys; hold Shift for fine control.");
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
