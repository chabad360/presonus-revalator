use crate::{ValueScale, snap_to_notch};
use egui::{
    Color32, DragValue, FontId, Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget,
};
use std::ops::RangeInclusive;

/// A vertical audio fader with linear, logarithmic, or audio-taper value mapping.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Fader<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    scale: ValueScale,
    text: Option<String>,
    prefix: String,
    suffix: String,
    size: Vec2,
    reset_value: Option<f32>,
    decimals: usize,
    decibels: bool,
    formatter: Option<Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>>,
    parser: Option<Box<dyn 'a + Fn(&str) -> Option<f64>>>,
    notches: Vec<f32>,
    notch_tolerance: f32,
}

impl<'a> Fader<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            scale: ValueScale::Linear,
            text: None,
            prefix: String::new(),
            suffix: String::new(),
            size: Vec2::new(96.0, 220.0),
            reset_value: None,
            decimals: 1,
            decibels: false,
            formatter: None,
            parser: None,
            notches: Vec::new(),
            notch_tolerance: 0.025,
        }
    }

    pub fn scale(mut self, scale: ValueScale) -> Self {
        self.scale = scale;
        self.decibels = false;
        self
    }

    /// Treats the stored value as decibels, applies an audio fader taper, and
    /// draws common mixer-fader tick marks.
    pub fn decibels(mut self) -> Self {
        self.scale = ValueScale::AudioTaper;
        self.decibels = true;
        self.suffix = " dB".to_owned();
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

    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.formatter = Some(Box::new(formatter));
        self
    }

    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    pub fn notch(mut self, value: f32) -> Self {
        self.notches.push(value);
        self
    }

    pub fn notches(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.notches.extend(values);
        self
    }

    pub fn notch_tolerance(mut self, tolerance: f32) -> Self {
        self.notch_tolerance = tolerance.max(0.0);
        self
    }
}

impl Widget for Fader<'_> {
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
            decibels,
            formatter,
            parser,
            notches,
            notch_tolerance,
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

        if response.clicked() {
            response.request_focus();
        }
        let fine = ui.input(|input| input.modifiers.shift);
        if response.double_clicked()
            && let Some(reset_value) = reset_value
        {
            let reset_value = reset_value.clamp(*range.start(), *range.end());
            if reset_value != *value {
                *value = reset_value;
                response.mark_changed();
            }
        } else if fine {
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                let normalized = scale.normalize(*value, &range) - delta.y / 800.0;
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
        } else if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let normalized = egui::remap_clamp(pointer.y, track.bottom()..=track.top(), 0.0..=1.0);
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
        painter.rect_filled(track, 3.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(track, 3.0, visuals.bg_stroke, egui::StrokeKind::Inside);

        let normalized = scale.normalize(*value, &range);
        let thumb_y = egui::lerp(track.bottom()..=track.top(), normalized);
        let filled = Rect::from_min_max(
            Pos2::new(track.left(), thumb_y),
            Pos2::new(track.right(), track.bottom()),
        );
        painter.rect_filled(filled, 3.0, ui.visuals().selection.bg_fill);

        if decibels {
            paint_decibel_ticks(ui, &painter, track, scale, &range);
        } else {
            paint_general_ticks(ui, &painter, track, scale, &range);
        }
        paint_notches(
            &painter,
            track,
            scale,
            &range,
            &notches,
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
            response = response.on_hover_text(
                "Drag the fader. Hold Shift for fine control, or click the value to type.",
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
