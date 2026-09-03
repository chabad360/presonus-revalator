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
    keyboard_step: Option<f32>,
    fine_keyboard_step: Option<f32>,
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
            keyboard_step: None,
            fine_keyboard_step: None,
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

    pub(crate) fn set_keyboard_step(&mut self, step: f32) {
        assert!(step.is_finite(), "keyboard step must be finite");
        self.keyboard_step = Some(step.abs());
    }

    pub(crate) fn set_fine_keyboard_step(&mut self, step: f32) {
        assert!(step.is_finite(), "fine keyboard step must be finite");
        self.fine_keyboard_step = Some(step.abs());
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

    /// Returns whether a double click outside `excluded_rect` was handled.
    pub(crate) fn reset_on_double_click(
        &mut self,
        ui: &Ui,
        widget_rect: Rect,
        excluded_rect: Rect,
        response: &mut Response,
    ) -> bool {
        let double_clicked = ui.input(|input| {
            input
                .pointer
                .button_double_clicked(egui::PointerButton::Primary)
                && input.pointer.interact_pos().is_some_and(|position| {
                    widget_rect.contains(position) && !excluded_rect.contains(position)
                })
        });
        if !double_clicked {
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

    fn keyboard_step_for(&self, fine: bool) -> f32 {
        let range_width = *self.range.end() - *self.range.start();
        let normal_step = self.keyboard_step.unwrap_or(range_width / 100.0);
        if fine {
            self.fine_keyboard_step.unwrap_or(normal_step / 10.0)
        } else {
            normal_step
        }
    }

    fn step_value(&mut self, direction: i8, fine: bool) -> bool {
        if direction == 0 {
            return false;
        }

        let step = self.keyboard_step_for(fine);
        let value = (self.value() + f32::from(direction) * step)
            .clamp(*self.range.start(), *self.range.end());
        if value == *self.value {
            false
        } else {
            *self.value = value;
            true
        }
    }

    pub(crate) fn handle_hover_input(
        &mut self,
        ui: &mut Ui,
        response: &mut Response,
        hovered: bool,
    ) {
        if !hovered {
            return;
        }

        let fine = ui.input(|input| input.modifiers.shift);
        let key_direction = ui.input(|input| {
            i8::from(input.key_pressed(Key::ArrowUp) || input.key_pressed(Key::ArrowRight))
                - i8::from(input.key_pressed(Key::ArrowDown) || input.key_pressed(Key::ArrowLeft))
        });
        let scroll_direction = ui.input_mut(|input| {
            let delta = input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::MouseWheel { delta, .. } => Some(*delta),
                    _ => None,
                })
                .fold(egui::Vec2::ZERO, |total, delta| total + delta);

            // Consume both the immediate delta and egui's derived smoothing so
            // the value changes exactly when wheel events arrive, rather than
            // continuing to tween over later frames.
            input.smooth_scroll_delta = egui::Vec2::ZERO;

            let dominant_delta = if delta.y.abs() >= delta.x.abs() {
                delta.y
            } else {
                delta.x
            };
            if dominant_delta > 0.0 {
                1
            } else if dominant_delta < 0.0 {
                -1
            } else {
                0
            }
        });

        if self.step_value(key_direction + scroll_direction, fine) {
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
    fn keyboard_steps_are_value_based_and_shift_uses_fine_step() {
        let mut value = 0.0;
        let mut control = ControlValue::new(&mut value, -96.0..=10.0);

        assert!((control.keyboard_step_for(false) - 1.06).abs() < f32::EPSILON);
        assert!((control.keyboard_step_for(true) - 0.106).abs() < f32::EPSILON);

        control.set_keyboard_step(0.5);
        control.set_fine_keyboard_step(0.05);
        assert_eq!(control.keyboard_step_for(false), 0.5);
        assert_eq!(control.keyboard_step_for(true), 0.05);
    }

    #[test]
    fn keyboard_steps_escape_magnetic_notches() {
        let mut value = 0.5;
        let mut control = ControlValue::new(&mut value, 0.0..=1.0);
        control.add_notch(0.5);
        control.set_keyboard_step(0.01);

        assert!(control.step_value(1, false));
        assert_eq!(control.value(), 0.51);
    }

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
