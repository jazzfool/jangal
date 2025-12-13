use iced::{
    advanced::{
        layout, overlay,
        widget::{Tree, tree},
    },
    widget::button,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Location {
    Auto,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub fn menu_button<'a, Message, Theme, Renderer>(
    content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
    menu_content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
) -> MenuButton<'a, Message, Theme, Renderer>
where
    Theme: button::Catalog,
    Renderer: iced::advanced::Renderer,
{
    MenuButton::new(content, menu_content)
}

pub struct MenuButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: button::Catalog,
    Renderer: iced::advanced::Renderer,
{
    content: iced::Element<'a, Message, Theme, Renderer>,
    menu_content: iced::Element<'a, Message, Theme, Renderer>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    width: iced::Length,
    height: iced::Length,
    location: Location,
    padding: iced::Padding,
    auto_close: bool,
    class: Theme::Class<'a>,
    status: button::Status,
}

impl<'a, Message, Theme, Renderer> MenuButton<'a, Message, Theme, Renderer>
where
    Theme: button::Catalog,
    Renderer: iced::advanced::Renderer,
{
    pub fn new(
        content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
        menu_content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let content = content.into();
        let menu_content = menu_content.into();
        let size = content.as_widget().size_hint();

        MenuButton {
            content,
            menu_content,
            on_toggle: None,
            width: size.width.fluid(),
            height: size.height.fluid(),
            location: Location::Auto,
            padding: iced::Padding::new(5.0).left(10.0).right(10.0),
            auto_close: true,
            class: Theme::default(),
            status: button::Status::Active,
        }
    }

    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn width(mut self, width: impl Into<iced::Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<iced::Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn location(mut self, location: Location) -> Self {
        self.location = location;
        self
    }

    pub fn padding<P: Into<iced::Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme, button::Status) -> button::Style + 'a) -> Self
    where
        Theme::Class<'a>: From<button::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as button::StyleFn<'a, Theme>).into();
        self
    }
}

struct State {
    is_open: bool,
}

impl<'a, Message, Theme, Renderer> iced::advanced::Widget<Message, Theme, Renderer>
    for MenuButton<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: button::Catalog,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State { is_open: false })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.menu_content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget(), self.menu_content.as_widget()]);
    }

    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let style = theme.style(&self.class, self.status);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                iced::advanced::renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                    snap: false,
                },
                style
                    .background
                    .unwrap_or(iced::Background::Color(iced::Color::TRANSPARENT)),
            );
        }

        let viewport = bounds.intersection(viewport).unwrap_or(*viewport);

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &iced::advanced::renderer::Style {
                text_color: style.text_color,
            },
            content_layout,
            cursor,
            &viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if state.is_open {
                    state.is_open = false;
                    if let Some(on_toggle) = &self.on_toggle {
                        shell.publish(on_toggle(false));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                } else if cursor.is_over(bounds) {
                    state.is_open = true;
                    if let Some(on_toggle) = &self.on_toggle {
                        shell.publish(on_toggle(true));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }

        let status = if cursor.is_over(bounds) {
            button::Status::Hovered
        } else {
            button::Status::Active
        };

        if let iced::Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            self.status = status;
        } else if self.status != status {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );

        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over {
            iced::mouse::Interaction::Pointer
        } else {
            iced::mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: iced::advanced::Layout<'_>,
        _renderer: &Renderer,
        _viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();

        state.is_open.then(|| {
            overlay::Element::new(Box::new(MenuButtonOverlay {
                tree,
                content: &mut self.menu_content,
                position: layout.position() + translation,
                content_size: layout.bounds().size(),
                location: self.location,
                auto_close: self.auto_close,
            }))
        })
    }
}

impl<'a, Message, Theme, Renderer> From<MenuButton<'a, Message, Theme, Renderer>>
    for iced::Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: button::Catalog + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(value: MenuButton<'a, Message, Theme, Renderer>) -> Self {
        iced::Element::new(value)
    }
}

struct MenuButtonOverlay<'a, 'b, Message, Theme, Renderer> {
    tree: &'b mut Tree,
    content: &'b mut iced::Element<'a, Message, Theme, Renderer>,
    position: iced::Point,
    content_size: iced::Size,
    location: Location,
    auto_close: bool,
}

impl<'a, 'b, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for MenuButtonOverlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: iced::Size) -> layout::Node {
        let mut layout = self.content.as_widget_mut().layout(
            &mut self.tree.children[1],
            renderer,
            &layout::Limits::new(iced::Size::new(0.0, 0.0), bounds),
        );
        layout.move_to_mut(self.position);
        let iced::Rectangle {
            x,
            y,
            width,
            height,
        } = layout.bounds();

        let x_offset = match (self.location, x + width > bounds.width) {
            (Location::Auto, true) | (Location::TopLeft, _) | (Location::BottomLeft, _) => {
                -width + self.content_size.width
            }
            (Location::Auto, false) | (Location::TopRight, _) | (Location::BottomRight, _) => 0.0,
        };
        let y_offset = match (self.location, y + height > bounds.height) {
            (Location::Auto, true) | (Location::TopLeft, _) | (Location::TopRight, _) => {
                -height - 5.0
            }
            (Location::Auto, false) | (Location::BottomLeft, _) | (Location::BottomRight, _) => {
                self.content_size.height + 5.0
            }
        };

        layout.translate_mut(iced::Vector::new(x_offset, y_offset));
        layout
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            &self.tree.children[1],
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        )
    }

    fn update(
        &mut self,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
    ) {
        self.content.as_widget_mut().update(
            &mut self.tree.children[1],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );

        if self.auto_close && shell.is_event_captured() {
            match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                    let state = self.tree.state.downcast_mut::<State>();
                    state.is_open = false;
                }
                _ => {}
            }
        }
    }

    fn mouse_interaction(
        &self,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &self.tree.children[1],
            layout,
            cursor,
            &iced::Rectangle::default(),
            renderer,
        )
    }
}
