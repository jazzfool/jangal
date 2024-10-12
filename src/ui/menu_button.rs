use iced::{
    advanced::{
        graphics::core::event,
        layout, overlay,
        widget::{tree, Tree},
    },
    widget::button,
};

pub fn menu_button<'a, Message, Theme, Renderer>(
    content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
    menu_content: impl Fn() -> iced::Element<'a, Message, Theme, Renderer> + 'static,
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
    menu_content: Box<dyn Fn() -> iced::Element<'a, Message, Theme, Renderer>>,
    width: iced::Length,
    height: iced::Length,
    padding: iced::Padding,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> MenuButton<'a, Message, Theme, Renderer>
where
    Theme: button::Catalog,
    Renderer: iced::advanced::Renderer,
{
    pub fn new(
        content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
        menu_content: impl Fn() -> iced::Element<'a, Message, Theme, Renderer> + 'static,
    ) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        MenuButton {
            content,
            menu_content: Box::new(menu_content),
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: iced::Padding::new(5.0).left(10.0).right(10.0),
            class: Theme::default(),
        }
    }

    pub fn width(mut self, width: impl Into<iced::Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<iced::Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding<P: Into<iced::Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
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
        vec![Tree::new(&self.content), Tree::new(&(self.menu_content)())]
    }

    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.content.as_widget().operate(
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
        let is_mouse_over = cursor.is_over(bounds);

        let status = if is_mouse_over {
            button::Status::Hovered
        } else {
            button::Status::Active
        };

        let style = theme.style(&self.class, status);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                iced::advanced::renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
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

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            return event::Status::Captured;
        }

        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if state.is_open {
                    state.is_open = false;
                    event::Status::Captured
                } else if cursor.is_over(bounds) {
                    state.is_open = true;
                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            _ => event::Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
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
        translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        state.is_open.then(|| {
            overlay::Element::new(Box::new(MenuButtonOverlay::new(
                tree,
                (self.menu_content)(),
                layout.position() + translation,
                layout.bounds().size(),
            )))
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

struct MenuButtonOverlay<'a, Message, Theme, Renderer> {
    tree: &'a mut Tree,
    content: iced::Element<'a, Message, Theme, Renderer>,
    position: iced::Point,
    content_size: iced::Size,
}

impl<'a, Message, Theme, Renderer> MenuButtonOverlay<'a, Message, Theme, Renderer> {
    pub fn new(
        tree: &'a mut Tree,
        content: iced::Element<'a, Message, Theme, Renderer>,
        position: iced::Point,
        content_size: iced::Size,
    ) -> Self {
        MenuButtonOverlay {
            tree,
            content,
            position,
            content_size,
        }
    }
}

impl<'a, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for MenuButtonOverlay<'a, Message, Theme, Renderer>
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
        layout.translate_mut(iced::Vector::new(
            if x + width > bounds.width {
                -width + self.content_size.width
            } else {
                0.0
            },
            if y + height > bounds.height {
                -height - 5.0
            } else {
                self.content_size.height + 5.0
            },
        ));
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

    fn on_event(
        &mut self,
        event: iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            &mut self.tree.children[1],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &self.tree.children[1],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
