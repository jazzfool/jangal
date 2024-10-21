use iced::{
    advanced::{self, image, layout, text, widget::tree},
    event, keyboard, mouse, touch,
};
use std::{marker::PhantomData, ops::RangeInclusive, time::Duration};

use crate::ui::MONO_FONT;

type Value = f64;

pub fn seekbar<'a, F, Message, Renderer>(
    range: RangeInclusive<Value>,
    duration: Duration,
    value: Value,
    thumbnails: Vec<image::Handle>,
    on_change: F,
) -> Seekbar<'a, Message, Renderer>
where
    F: 'a + Fn(Value) -> Message,
    Renderer: text::Renderer<Font = iced::Font>
        + image::Renderer<Handle = image::Handle>
        + advanced::Renderer,
{
    Seekbar::new(range, duration, value, thumbnails, on_change)
}

pub struct Seekbar<'a, Message, Renderer = iced::Renderer> {
    range: RangeInclusive<Value>,
    duration: Duration,
    step: Value,
    value: Value,
    on_change: Box<dyn Fn(Value) -> Message + 'a>,
    on_release: Option<Message>,
    width: iced::Length,
    height: f32,
    thumbnails: Vec<image::Handle>,
    _renderer: PhantomData<Renderer>,
}

impl<'a, Message, Renderer> Seekbar<'a, Message, Renderer> {
    pub fn new<F>(
        range: RangeInclusive<Value>,
        duration: Duration,
        value: Value,
        thumbnails: Vec<image::Handle>,
        on_change: F,
    ) -> Self
    where
        F: 'a + Fn(Value) -> Message,
    {
        Seekbar {
            range,
            duration,
            value,
            step: Value::from(1),
            on_change: Box::new(on_change),
            on_release: None,
            width: iced::Length::Fill,
            height: 20.0,
            thumbnails,
            _renderer: Default::default(),
        }
    }

    pub fn on_release(mut self, on_release: Message) -> Self {
        self.on_release = Some(on_release);
        self
    }

    pub fn step(mut self, step: impl Into<Value>) -> Self {
        self.step = step.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct State {
    is_hovered: bool,
    cursor_position: iced::Point,
    cursor_location: Value,
    is_dragging: bool,
    keyboard_modifiers: keyboard::Modifiers,
}

impl<'a, Message, Theme, Renderer> advanced::Widget<Message, Theme, Renderer>
    for Seekbar<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = iced::Font>
        + image::Renderer<Handle = image::Handle>
        + advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: self.width,
            height: iced::Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn on_event(
        &mut self,
        tree: &mut tree::Tree,
        event: iced::Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        let is_dragging = state.is_dragging;
        let current_value = self.value;

        let locate = |cursor_position: iced::Point| -> Value {
            let bounds = layout.bounds();
            let new_value = if cursor_position.x <= bounds.x {
                *self.range.start()
            } else if cursor_position.x >= bounds.x + bounds.width {
                *self.range.end()
            } else {
                let start = *self.range.start();
                let end = *self.range.end();

                let percent = f64::from(cursor_position.x - bounds.x) / f64::from(bounds.width);

                let steps = (percent * (end - start) / self.step).round();
                let value = steps * self.step + start;

                value.min(end)
            };

            new_value
        };

        let increment = |value: Value| -> Value {
            let steps = (value / self.step).round();
            let new_value = self.step * (steps + 1.0);

            if new_value > (*self.range.end()).into() {
                return *self.range.end();
            }

            new_value
        };

        let decrement = |value: Value| -> Value {
            let steps = (value / self.step).round();
            let new_value = self.step * (steps - 1.0);

            if new_value < (*self.range.start()).into() {
                return *self.range.start();
            }

            new_value
        };

        let mut change = |new_value: Value| {
            if (self.value - new_value).abs() > f64::EPSILON {
                shell.publish((self.on_change)(new_value));
                self.value = new_value;
            }
        };

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) = cursor.position_over(layout.bounds()) {
                    let _ = change(locate(cursor_position));
                    state.is_dragging = true;

                    return event::Status::Captured;
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerLifted { .. })
            | iced::Event::Touch(touch::Event::FingerLost { .. }) => {
                if is_dragging {
                    if let Some(on_release) = self.on_release.clone() {
                        shell.publish(on_release);
                    }
                    state.is_dragging = false;

                    return event::Status::Captured;
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. })
            | iced::Event::Touch(touch::Event::FingerMoved { .. }) => {
                state.is_hovered = cursor
                    .position()
                    .map(|pos| layout.bounds().contains(pos))
                    .unwrap_or(false);
                state.cursor_position = cursor.position().unwrap_or_default();
                state.cursor_location = cursor.position().map(locate).unwrap_or_default();

                if is_dragging {
                    let _ = cursor.position().map(locate).map(change);

                    return event::Status::Captured;
                }
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta })
                if state.keyboard_modifiers.control() =>
            {
                if cursor.is_over(layout.bounds()) {
                    let delta = match delta {
                        mouse::ScrollDelta::Lines { x: _, y } => y,
                        mouse::ScrollDelta::Pixels { x: _, y } => y,
                    };

                    if delta < 0.0 {
                        change(decrement(current_value));
                    } else {
                        change(increment(current_value));
                    }

                    return event::Status::Captured;
                }
            }
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if cursor.is_over(layout.bounds()) {
                    match key {
                        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                            change(increment(current_value));
                        }
                        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                            change(decrement(current_value));
                        }
                        _ => (),
                    }

                    return event::Status::Captured;
                }
            }
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = modifiers;
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        _tree: &tree::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &advanced::renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();

        let rail_backgrounds = (
            iced::Background::Color(iced::Color::from_rgba8(245, 245, 245, 0.9)),
            iced::Background::Color(iced::Color::from_rgba8(150, 150, 150, 0.7)),
        );
        let rail_width = 3.0;

        let handle_background =
            iced::Background::Color(iced::Color::from_rgba8(245, 245, 245, 0.9));
        let handle_width = 7.0;
        let handle_height = bounds.height;
        let handle_border_radius = iced::border::radius(2.0);

        let value = self.value as f32;
        let (range_start, range_end) = {
            let (start, end) = self.range.clone().into_inner();

            (start as f32, end as f32)
        };

        let offset = if range_start >= range_end {
            0.0
        } else {
            (bounds.width - handle_width) * (value - range_start) / (range_end - range_start)
        };

        let rail_y = bounds.y + bounds.height / 2.0;
        let rail_gap = 5.5;

        renderer.fill_quad(
            advanced::renderer::Quad {
                bounds: iced::Rectangle {
                    x: bounds.x,
                    y: rail_y - rail_width / 2.0,
                    width: offset + handle_width / 2.0 - rail_gap,
                    height: rail_width,
                },
                border: Default::default(),
                ..advanced::renderer::Quad::default()
            },
            rail_backgrounds.0,
        );

        renderer.fill_quad(
            advanced::renderer::Quad {
                bounds: iced::Rectangle {
                    x: bounds.x + offset + handle_width / 2.0 + rail_gap,
                    y: rail_y - rail_width / 2.0,
                    width: bounds.width - offset - handle_width / 2.0 - rail_gap,
                    height: rail_width,
                },
                border: Default::default(),
                ..advanced::renderer::Quad::default()
            },
            rail_backgrounds.1,
        );

        renderer.fill_quad(
            advanced::renderer::Quad {
                bounds: iced::Rectangle {
                    x: bounds.x + offset,
                    y: rail_y - handle_height / 2.0,
                    width: handle_width,
                    height: handle_height,
                },
                border: iced::Border {
                    radius: handle_border_radius,
                    width: 0.0,
                    color: iced::Color::TRANSPARENT,
                },
                ..advanced::renderer::Quad::default()
            },
            handle_background,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if is_mouse_over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();

        let cursor_percent =
            (state.cursor_location - *self.range.start()) / (self.range.end() - self.range.start());
        let image_size = renderer.measure_image(&self.thumbnails[0]);
        let image_index = (cursor_percent * self.thumbnails.len() as Value) as usize;
        let image_index = image_index.min(self.thumbnails.len());

        let position = (cursor_percent * self.duration.as_secs_f64()) as u64;
        let timestamp = format!(
            "{:02}:{:02}:{:02}",
            position as u64 / 3600,
            position as u64 % 3600 / 60,
            position as u64 % 60
        );

        (state.is_hovered || state.is_dragging).then(|| {
            advanced::overlay::Group::with_children(vec![
                advanced::overlay::Element::new(Box::new(ThumbnailOverlay {
                    position: layout.position() + translation,
                    content_bounds: layout.bounds(),
                    image: self.thumbnails[image_index].clone(),
                    image_size: iced::Size::new(
                        image_size.width as f32 / image_size.height as f32 * 100.0,
                        100.0,
                    ),
                    cursor_position: state.cursor_position,
                })),
                advanced::overlay::Element::new(Box::new(TimestampOverlay {
                    position: layout.position() + translation,
                    content_bounds: layout.bounds(),
                    timestamp,
                    size: iced::Size::new(80.0, 20.0),
                    cursor_position: state.cursor_position,
                })),
            ])
            .overlay()
        })
    }
}

impl<'a, Message, Theme, Renderer> From<Seekbar<'a, Message, Renderer>>
    for iced::Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = iced::Font>
        + image::Renderer<Handle = image::Handle>
        + advanced::Renderer
        + 'a,
{
    fn from(value: Seekbar<'a, Message, Renderer>) -> Self {
        iced::Element::new(value)
    }
}

struct ThumbnailOverlay {
    position: iced::Point,
    content_bounds: iced::Rectangle,
    image: image::Handle,
    image_size: iced::Size,
    cursor_position: iced::Point,
}

impl<Message, Theme, Renderer> advanced::Overlay<Message, Theme, Renderer> for ThumbnailOverlay
where
    Renderer: image::Renderer<Handle = image::Handle> + advanced::Renderer,
{
    fn layout(&mut self, _renderer: &Renderer, _bounds: iced::Size) -> layout::Node {
        let translation = self.position - self.content_bounds.position();
        let position = iced::Vector::new(
            self.cursor_position.x - self.image_size.width / 2.0,
            self.position.y - self.image_size.height - 30.0,
        ) + translation;

        layout::Node::new(self.image_size).translate(position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &advanced::renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
    ) {
        renderer.fill_quad(
            advanced::renderer::Quad {
                bounds: layout.bounds().expand(2.0),
                border: iced::Border::default().rounded(3.0),
                shadow: iced::Shadow {
                    color: iced::Color::BLACK.scale_alpha(1.2),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 20.0,
                },
            },
            iced::Background::Color(iced::Color::WHITE.scale_alpha(0.5)),
        );
        renderer.draw_image(image::Image::new(self.image.clone()), layout.bounds());
    }
}

struct TimestampOverlay {
    position: iced::Point,
    content_bounds: iced::Rectangle,
    size: iced::Size,
    timestamp: String,
    cursor_position: iced::Point,
}

impl<Message, Theme, Renderer> advanced::Overlay<Message, Theme, Renderer> for TimestampOverlay
where
    Renderer: text::Renderer<Font = iced::Font>,
{
    fn layout(&mut self, _renderer: &Renderer, _bounds: iced::Size) -> layout::Node {
        let translation = self.position - self.content_bounds.position();
        let position = iced::Vector::new(
            self.cursor_position.x - self.size.width / 2.0,
            self.position.y - self.size.height - 5.0,
        ) + translation;

        layout::Node::new(self.size).translate(position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &advanced::renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
    ) {
        renderer.fill_quad(
            advanced::renderer::Quad {
                bounds: layout.bounds(),
                border: iced::Border::default().rounded(3.0),
                shadow: Default::default(),
            },
            iced::Background::Color(iced::Color::BLACK.scale_alpha(0.9)),
        );

        renderer.fill_text(
            text::Text {
                content: self.timestamp.clone(),
                bounds: self.size,
                size: 13.into(),
                line_height: Default::default(),
                font: MONO_FONT,
                horizontal_alignment: iced::Alignment::Center.into(),
                vertical_alignment: iced::Alignment::Center.into(),
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
            },
            layout.bounds().center() + iced::Vector::new(0.0, 1.5),
            iced::Color::from_rgb8(210, 210, 210),
            layout.bounds(),
        );
    }
}
