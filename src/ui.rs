pub mod app;
pub mod menu_button;
pub mod screen;

pub use menu_button::menu_button;

use crate::{library, settings::UserSettings};
use iced::widget::{button, container, scrollable, text, text_input};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LibraryStatus {
    Idle,
    Scanning,
}

pub struct AppState {
    pub storage_path: PathBuf,
    pub library: library::Library,
    pub settings: UserSettings,

    pub library_status: LibraryStatus,
}

pub const SANS_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Work Sans"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const HEADER_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Lexend"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const MONO_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Courier Prime"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const ICON_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Material Symbols Sharp Filled"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", text[..max_len - 3].trim_end())
    } else {
        text.into()
    }
}

pub fn clear_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Active | button::Status::Disabled => iced::Color::TRANSPARENT,
            _ => iced::Color::from_rgba8(255, 255, 255, 0.03),
        })),
        border: iced::Border::default().rounded(5.0),
        text_color: match status {
            button::Status::Disabled => theme.extended_palette().background.strong.color,
            _ => theme.palette().text,
        },
        ..Default::default()
    }
}

pub fn flat_text_input(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(match status {
            text_input::Status::Focused => iced::Color::from_rgb8(10, 10, 10),
            _ => iced::Color::from_rgb8(50, 50, 50),
        }),
        border: iced::Border::default()
            .rounded(5.0)
            .color(iced::Color::from_rgb8(30, 30, 30))
            .width(match status {
                text_input::Status::Focused => 1.0,
                _ => 0.0,
            }),
        icon: theme.palette().text,
        placeholder: iced::Color::from_rgb8(130, 130, 130),
        value: theme.palette().text,
        selection: theme.palette().primary,
    }
}

pub fn clear_scrollable(theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    fn clear_rail(hover: bool, dragged: bool) -> scrollable::Rail {
        scrollable::Rail {
            background: None,
            border: Default::default(),
            scroller: scrollable::Scroller {
                color: iced::Color::WHITE.scale_alpha(if dragged {
                    0.6
                } else if hover {
                    0.4
                } else {
                    0.2
                }),
                border: iced::Border::default()
                    .rounded(5.0)
                    .color(iced::Color::TRANSPARENT)
                    .width(2.0),
            },
        }
    }

    let (hover_vertical, hover_horizontal) = match status {
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
        } => (
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
        ),
        _ => (false, false),
    };

    let (drag_vertical, drag_horizontal) = match status {
        scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
        } => (
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
        ),
        _ => (false, false),
    };

    scrollable::Style {
        container: container::Style {
            text_color: None,
            background: None,
            border: Default::default(),
            shadow: iced::Shadow {
                color: iced::Color::TRANSPARENT,
                offset: iced::Vector::ZERO,
                blur_radius: 0.0,
            },
        },
        vertical_rail: clear_rail(hover_vertical, drag_vertical),
        horizontal_rail: clear_rail(hover_horizontal, drag_horizontal),
        gap: None,
    }
}

pub fn icon<'a>(codepoint: u32) -> iced::widget::Text<'a> {
    text(char::from_u32(codepoint).expect("valid icon codepoint")).font(ICON_FONT)
}
