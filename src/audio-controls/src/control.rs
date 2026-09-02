use crate::{ValueScale, snap_to_notch};
use egui::{DragValue, Id, Key, Rect, Response, Ui};
use std::ops::RangeInclusive;

pub(crate) type ValueFormatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;
pub(crate) type ValueParser<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;

pub(crate) struct ControlValue<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    scale: ValueScale,
    prefix: String,
    suffix: String,
    decimals: usize,
    formatter: Option<ValueFormatter<'a>>,
    parser: Option<ValueParser<'a>>,
    notches: Vec<f32>,
    notch_tolerance: f32,
    reset_value: Option<f32>,
}

impl<'a> ControlValue<'a> {
    pub(crate) fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            scale: ValueScale::Linear,
            prefix: String::new(),
            suffix: String::new(),
            decimals: 1,
            formatter: None,
            parser: None,
            notches: Vec::new(),
            notch_tolerance: 0.025,
            reset_value: None,
        }
    }

    pub(crate) fn value(&self) -> f32 {
        *self.value
    }

    pub(crate) fn range(&self) -> &RangeInclusive<f32> {
        &self.range
    }

    pub(crate) fn scale(&self) -> ValueScale {
        self.scale
    }

    pub(crate) fn notches(&self) -> &[f32] {
        &self.notches
    }

    pub(crate) fn set_scale(&mut self, scale: ValueScale) {
        self.scale = scale;
    }

    pub(crate) fn set_prefix(&mut self, prefix: String) {
        self.prefix = prefix;
    }

    pub(crate) fn set_suffix(&mut self, suffix: String) {
        self.suffix = suffix;
    }

    pub(crate) fn set_decimals(&mut self, decimals: usize) {
        self.decimals = decimals;
    }

    pub(crate) fn set_formatter(
        &mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) {
        self.formatter = Some(Box::new(formatter));
    }

    pub(crate) fn set_parser(&mut self, parser: impl 'a + Fn(&str) -> Option<f64>) {
        self.parser = Some(Box::new(parser));
    }

    pub(crate) fn add_notch(&mut self, value: f32) {
        self.notches.push(value);
    }

    pub(crate) fn add_notches(&mut self, values: impl IntoIterator<Item = f32>) {
        self.notches.extend(values);
    }

    pub(crate) fn set_notch_tolerance(&mut self, tolerance: f32) {
        self.notch_tolerance = tolerance.max(0.0);
    }

    pub(crate) fn set_reset_value(&mut self, value: f32) {
        self.reset_value = Some(value);
    }

    pub(crate) fn normalized_value(&self) -> f32 {
        self.scale.normalize(self.value(), &self.range)
    }

    pub(crate) fn set_normalized_value(&mut self, normalized: f32) -> bool {
        let value = snap_to_notch(
            self.scale.denormalize(normalized, &self.range),
            self.scale,
            &self.range,
            &self.notches,
            self.notch_tolerance,
        );
        if value == *self.value {
            false
        } else {
            *self.value = value;
            true
        }
    }

    pub(crate) fn update_normalized_value(&mut self, normalized: f32, response: &mut Response) {
        if self.set_normalized_value(normalized) {
            response.mark_changed();
        }
    }

    pub(crate) fn focus_on_click(&self, response: &Response) {
        if response.clicked() {
            response.request_focus();
        }
    }

    /// Returns whether the double click was handled, so callers can suppress drag handling.
    pub(crate) fn reset_on_double_click(&mut self, response: &mut Response) -> bool {
        if !response.double_clicked() {
            return false;
        }
        let Some(reset_value) = self.reset_value else {
            return false;
        };

        let reset_value = reset_value.clamp(*self.range.start(), *self.range.end());
        if reset_value != *self.value {
            *self.value = reset_value;
            response.mark_changed();
        }
        true
    }

    pub(crate) fn handle_keyboard(&mut self, ui: &Ui, response: &mut Response, fine: bool) {
        if !response.has_focus() {
            return;
        }

        let direction = ui.input(|input| {
            i8::from(input.key_pressed(Key::ArrowUp) || input.key_pressed(Key::ArrowRight))
                - i8::from(input.key_pressed(Key::ArrowDown) || input.key_pressed(Key::ArrowLeft))
        });
        if direction != 0 {
            let step = if fine { 0.001 } else { 0.01 };
            let normalized = self.normalized_value() + f32::from(direction) * step;
            self.set_normalized_value(normalized);
            response.mark_changed();
        }
    }

    pub(crate) fn add_editable(
        &mut self,
        ui: &mut Ui,
        id: Id,
        value_rect: Rect,
        control_response: &mut Response,
    ) -> Response {
        let mut value_widget = DragValue::new(&mut *self.value)
            .range(self.range.clone())
            .fixed_decimals(self.decimals)
            .prefix(std::mem::take(&mut self.prefix))
            .suffix(std::mem::take(&mut self.suffix));
        if let Some(formatter) = self.formatter.take() {
            value_widget = value_widget.custom_formatter(formatter);
        }
        if let Some(parser) = self.parser.take() {
            value_widget = value_widget.custom_parser(parser);
        }

        let mut value_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id)
                .max_rect(value_rect)
                .layout(egui::Layout::centered_and_justified(
                    egui::Direction::LeftToRight,
                )),
        );
        let value_response = value_ui.add_sized(value_rect.size(), value_widget);
        if value_response.changed() {
            control_response.mark_changed();
        }
        value_response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_updates_snap_to_nearby_notches() {
        let mut value = 0.0;
        let mut control = ControlValue::new(&mut value, 0.0..=1.0);
        control.add_notch(0.5);

        assert!(control.set_normalized_value(0.51));
        assert_eq!(control.value(), 0.5);
        assert!(!control.set_normalized_value(0.51));
        assert_eq!(control.value(), 0.5);
        assert!(control.set_normalized_value(0.6));
        assert_eq!(control.value(), 0.6);
    }

    #[test]
    fn negative_notch_tolerance_disables_nearby_snapping() {
        let mut value = 0.0;
        let mut control = ControlValue::new(&mut value, 0.0..=1.0);
        control.add_notch(0.5);
        control.set_notch_tolerance(-1.0);

        assert!(control.set_normalized_value(0.51));
        assert_eq!(control.value(), 0.51);
    }
}
