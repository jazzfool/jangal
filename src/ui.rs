pub mod app;
pub mod menu_button;
pub mod screen;

pub use menu_button::menu_button;

use crate::{library, settings::UserSettings};
use iced::widget::{button, checkbox, container, row, scrollable, text, text_input};
use std::{
    collections::VecDeque,
    future::Future,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tab {
    Home,
    Movies,
    TvShows,
    TvShow(library::MediaId),
    Season(library::MediaId),
    Collection(library::CollectionId),
}

impl Tab {
    pub fn overwrites(&self, other: &Tab) -> bool {
        matches!(
            self,
            Tab::Home | Tab::Movies | Tab::TvShows | Tab::Collection(_)
        ) && matches!(
            other,
            Tab::Home | Tab::Movies | Tab::TvShows | Tab::Collection(_)
        )
    }
}

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
    pub tab_stack: VecDeque<Tab>,
}

impl AppState {
    pub fn save_library<'a, 'b>(&'a self) -> impl Future<Output = anyhow::Result<()>> + 'b {
        let library = self.library.clone();
        let storage_path = self.storage_path.clone();
        async move {
            library.save(&storage_path)?;
            Ok(())
        }
    }

    pub fn save_settings<'a, 'b>(&'a self) -> impl Future<Output = anyhow::Result<()>> + 'b {
        let settings = self.settings.clone();
        let storage_path = self.storage_path.clone();
        async move {
            settings.save(&storage_path)?;
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub const SANS_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Segoe UI"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

#[cfg(target_os = "macos")]
pub const SANS_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("SF Pro"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

#[cfg(target_os = "linux")]
pub const SANS_FONT: iced::Font = iced::Font {
    family: iced::font::Family::SansSerif,
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const HEADER_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Work Sans"),
    weight: iced::font::Weight::Medium,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const MONO_FONT: iced::Font = iced::Font::MONOSPACE;

pub const ICON_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Material Symbols Rounded Filled 28pt"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const SUBTITLE_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Nimbus Sans"),
    weight: iced::font::Weight::Bold,
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

pub fn greyscale(rgb: u8) -> iced::Color {
    iced::Color::from_rgb8(rgb, rgb, rgb)
}

#[cfg(target_os = "windows")]
const OPEN_CMD: &str = "explorer";

#[cfg(target_os = "macos")]
const OPEN_CMD: &str = "open";

#[cfg(target_os = "linux")]
const OPEN_CMD: &str = "xdg-open";

pub fn open_path(p: impl AsRef<Path>) {
    let _ = Command::new(OPEN_CMD).arg(p.as_ref().as_os_str()).spawn();
}

pub fn themed_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Active | button::Status::Disabled => iced::Color::TRANSPARENT,
            _ => iced::Color::from_rgba8(255, 255, 255, 0.1),
        })),
        border: iced::Border::default().rounded(5.0),
        text_color: match status {
            button::Status::Disabled => theme.extended_palette().background.strong.color,
            _ => theme.palette().text,
        },
        ..Default::default()
    }
}

pub fn themed_text_input(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(match status {
            text_input::Status::Focused { .. } => iced::Color::from_rgb8(10, 10, 10),
            _ => iced::Color::from_rgb8(50, 50, 50),
        }),
        border: iced::Border::default()
            .rounded(5.0)
            .color(iced::Color::from_rgb8(30, 30, 30))
            .width(match status {
                text_input::Status::Focused { .. } => 1.0,
                _ => 0.0,
            }),
        icon: theme.palette().text,
        placeholder: iced::Color::from_rgb8(130, 130, 130),
        value: theme.palette().text,
        selection: theme.palette().primary,
    }
}

pub fn themed_scrollable(_theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    fn themed_rail(hover: bool, dragged: bool) -> scrollable::Rail {
        scrollable::Rail {
            background: None,
            border: Default::default(),
            scroller: scrollable::Scroller {
                background: iced::Background::Color(iced::Color::WHITE.scale_alpha(if dragged {
                    0.6
                } else if hover {
                    0.4
                } else {
                    0.2
                })),
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
            ..
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
            ..
        } => (
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
        ),
        _ => (false, false),
    };

    scrollable::Style {
        container: container::Style {
            text_color: None,
            background: Some(iced::Background::Color(iced::Color::BLACK)),
            border: Default::default(),
            shadow: iced::Shadow {
                color: iced::Color::TRANSPARENT,
                offset: iced::Vector::ZERO,
                blur_radius: 0.0,
            },
            snap: false,
        },
        vertical_rail: themed_rail(hover_vertical, drag_vertical),
        horizontal_rail: themed_rail(hover_horizontal, drag_horizontal),
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: iced::Background::Color(iced::Color::WHITE.scale_alpha(0.6)),
            border: Default::default(),
            shadow: iced::Shadow {
                color: iced::Color::TRANSPARENT,
                offset: iced::Vector::ZERO,
                blur_radius: 0.0,
            },
            icon: iced::Color::WHITE.scale_alpha(0.6),
        },
    }
}

pub fn themed_menu(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(iced::Color::BLACK)),
        border: iced::Border {
            color: theme.extended_palette().background.weak.color,
            width: 1.0,
            radius: iced::border::radius(10.0),
        },
        shadow: iced::Shadow {
            color: iced::Color::BLACK.scale_alpha(0.8),
            offset: iced::Vector::new(0.0, 3.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

pub fn icon<'a>(codepoint: u32) -> iced::widget::Text<'a> {
    text(char::from_u32(codepoint).expect("valid icon codepoint")).font(ICON_FONT)
}

pub fn themed_checkbox(_theme: &iced::Theme, status: checkbox::Status) -> checkbox::Style {
    let is_checked = match status {
        checkbox::Status::Active { is_checked }
        | checkbox::Status::Hovered { is_checked }
        | checkbox::Status::Disabled { is_checked } => is_checked,
    };

    checkbox::Style {
        background: match status {
            checkbox::Status::Active { .. } => {
                iced::Background::Color(greyscale(0).scale_alpha(0.9))
            }
            checkbox::Status::Hovered { .. } => iced::Background::Color(greyscale(0)),
            checkbox::Status::Disabled { .. } => {
                iced::Background::Color(greyscale(0).scale_alpha(0.5))
            }
        },
        icon_color: if is_checked {
            greyscale(200)
        } else {
            iced::Color::TRANSPARENT
        },
        border: iced::Border {
            color: greyscale(100).scale_alpha(0.25),
            width: 1.0,
            radius: iced::border::radius(3.0),
        },
        text_color: None,
    }
}

pub fn rich_checkbox<'a, Message>(
    label: impl Into<iced::Element<'a, Message>>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + Clone + 'a,
) -> iced::Element<'a, Message>
where
    Message: 'a + Clone,
{
    button(
        row![]
            .spacing(5.0)
            .push(
                checkbox(is_checked)
                    .style(themed_checkbox)
                    .on_toggle(on_toggle.clone()),
            )
            .push(label),
    )
    .on_press_with(move || on_toggle(!is_checked))
    .style(themed_button)
    .into()
}

pub fn find_focused_maybe()
-> impl iced::advanced::widget::Operation<Option<iced::advanced::widget::Id>> {
    use iced::advanced::widget::{
        Id, Operation,
        operation::{Focusable, Outcome},
    };

    struct FindFocused {
        focused: Option<Id>,
    }

    impl Operation<Option<Id>> for FindFocused {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Option<Id>>),
        ) {
            operate(self);
        }

        fn focusable(
            &mut self,
            id: Option<&Id>,
            _bounds: iced::Rectangle,
            state: &mut dyn Focusable,
        ) {
            if state.is_focused() && id.is_some() {
                self.focused = id.cloned();
            }
        }

        fn finish(&self) -> Outcome<Option<Id>> {
            Outcome::Some(self.focused.clone())
        }
    }

    FindFocused { focused: None }
}
