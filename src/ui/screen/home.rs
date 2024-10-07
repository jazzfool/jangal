use super::Screen;
use crate::{
    library,
    ui::{clear_button, flat_text_input, AppState, LibraryStatus, ICON_FONT, SANS_FONT},
};
use iced::widget::{
    button, center, column, container, horizontal_rule, horizontal_space, image, row, rule,
    scrollable, stack, text, text_input, vertical_rule, vertical_space,
};
use itertools::Itertools;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Tab {
    Movies,
    TvShows,
    TvShow(library::MediaId),
    Season(library::MediaId),
}

pub struct Home {
    search: String,
    tab: Tab,
}

impl Home {
    pub fn new() -> (Self, iced::Task<HomeMessage>) {
        (
            Home {
                search: String::new(),
                tab: Tab::Movies,
            },
            iced::Task::none(),
        )
    }
}

impl Screen for Home {
    type Message = HomeMessage;

    fn update(&mut self, message: HomeMessage, state: &mut AppState) -> iced::Task<HomeMessage> {
        match message {
            HomeMessage::Search(value) => {
                self.search = value;
                iced::Task::none()
            }
            HomeMessage::ScanDirectories => {
                state.library_status = LibraryStatus::Scanning;

                let existing: Vec<_> = state
                    .library
                    .iter()
                    .filter_map(|(id, media)| Some((*id, media.path()?.to_path_buf())))
                    .collect();

                let directories = state.settings.directories.clone();

                iced::Task::perform(
                    async move {
                        let removed = library::purge_media(existing.into_iter()).await;
                        let added = library::scan_directories(
                            directories.iter().map(|path| path.as_path()),
                        )
                        .await
                        .unwrap_or_default();
                        (removed, added)
                    },
                    |(removed, added)| HomeMessage::ScanDirectoriesComplete { removed, added },
                )
            }
            HomeMessage::Goto(tab) => {
                self.tab = tab;
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view(&self, state: &AppState) -> iced::Element<HomeMessage> {
        row![]
            .push(sidebar(state.library_status))
            .push(vertical_rule(1.0).style(|theme| rule::Style {
                color: iced::Color::from_rgb8(40, 40, 40),
                ..<iced::Theme as rule::Catalog>::default()(theme)
            }))
            .push(
                stack![]
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .push(
                        container(
                            scrollable(
                                center(filtered_media_list(
                                    &self.search,
                                    state.library.iter().filter(|(_, media)| match self.tab {
                                        Tab::Movies => matches!(media, library::Media::Movie(_)),
                                        Tab::TvShows => matches!(media, library::Media::Series(_)),
                                        _ => unreachable!(),
                                    }),
                                ))
                                .height(iced::Length::Shrink)
                                .align_y(iced::Alignment::Start)
                                .padding(iced::Padding::ZERO.left(20.0).right(20.0)),
                            )
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .direction(
                                scrollable::Direction::Vertical(scrollable::Scrollbar::new()),
                            ),
                        )
                        .clip(true)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .padding(iced::Padding::ZERO.top(70.0)),
                    )
                    .push(
                        column![]
                            .width(iced::Length::Fill)
                            .push(top_bar(&self.search, self.tab, &state.library))
                            .push(horizontal_rule(1.0).style(|theme| rule::Style {
                                color: iced::Color::from_rgb8(40, 40, 40),
                                ..<iced::Theme as rule::Catalog>::default()(theme)
                            })),
                    ),
            )
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum HomeMessage {
    SelectMovie(library::MediaId),
    ScrapeComplete((library::MediaId, library::MovieMetadata)),
    OpenSettings,
    Search(String),
    ScanDirectories,
    ScanDirectoriesComplete {
        removed: Vec<library::MediaId>,
        added: Vec<library::Media>,
    },
    Goto(Tab),
}

fn media_list<'a, 'b>(
    media: impl Iterator<Item = (&'b library::MediaId, &'b library::Media)>,
) -> iced::Element<'a, HomeMessage> {
    row![]
        .spacing(10.0)
        .padding(iced::Padding::new(0.0).top(20.0).bottom(20.0))
        .clip(true)
        .extend(media.map(|(id, media)| media_card(*id, media)))
        .wrap()
        .into()
}

fn filtered_media_list<'a, 'b>(
    search: &str,
    media: impl Iterator<Item = (&'b library::MediaId, &'b library::Media)>,
) -> iced::Element<'a, HomeMessage> {
    if search.trim().is_empty() {
        media_list(media.sorted_by(|(_, a), (_, b)| a.title().cmp(&b.title())))
    } else {
        media_list(
            media
                .map(|(id, media)| {
                    (
                        id,
                        media,
                        sublime_fuzzy::best_match(search, &media.full_title().unwrap_or_default()),
                    )
                })
                .filter_map(|(id, media, fuzzy)| Some((id, media, fuzzy?.score())))
                .sorted_by(|(_, _, a), (_, _, b)| a.cmp(b))
                .map(|(id, media, _)| (id, media)),
        )
    }
}

fn media_card<'a>(id: library::MediaId, media: &library::Media) -> iced::Element<'a, HomeMessage> {
    let has_poster = media.poster().is_some();
    column![]
        .spacing(5.0)
        .width(150.0)
        .clip(true)
        .push(
            container(if let Some(img) = media.poster() {
                image(img)
                    .content_fit(iced::ContentFit::Cover)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .into()
            } else {
                iced::Element::from("")
            })
            .width(iced::Length::Fill)
            .height(225.0)
            .clip(true)
            .style(move |_| container::Style {
                background: (!has_poster).then_some(iced::Background::Color(iced::Color::WHITE)),
                border: iced::Border {
                    radius: iced::border::Radius::new(5.0),
                    ..Default::default()
                },
                shadow: iced::Shadow {
                    color: iced::Color::BLACK,
                    offset: iced::Vector::new(0.0, 3.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            }),
        )
        .push(
            text(media.full_title().unwrap_or(String::from("Unknown Media")))
                .wrapping(text::Wrapping::None)
                .size(14.0)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..SANS_FONT
                }),
        )
        .push(button("Watch").on_press(HomeMessage::SelectMovie(id)))
        .into()
}

fn top_bar<'a>(
    search: &str,
    tab: Tab,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    container(
        row![]
            .width(iced::Length::Fill)
            .height(iced::Length::Shrink)
            .spacing(10.0)
            .align_y(iced::Alignment::Center)
            .push(
                text(match tab {
                    Tab::Movies => "Movies".into(),
                    Tab::TvShows => "TV Shows".into(),
                    Tab::TvShow(id) => library.get(id).unwrap().title().unwrap(),
                    Tab::Season(id) => {
                        let library::Media::Season(season) = library.get(id).unwrap() else {
                            panic!()
                        };
                        let series = library.get(season.series).unwrap();
                        format!(
                            "{} S{:02} - {}",
                            series.title().unwrap(),
                            season.metadata.season,
                            season.metadata.title
                        )
                    }
                })
                .size(26.0)
                .color(iced::Color::from_rgba8(210, 210, 210, 1.0)),
            )
            .push(horizontal_space())
            .push(
                text_input("Search...", search)
                    .on_input(HomeMessage::Search)
                    .width(200.0)
                    .padding(iced::Padding::new(5.0).left(10.0))
                    .icon(text_input::Icon {
                        font: ICON_FONT,
                        code_point: char::from_u32(0xe8b6).unwrap(),
                        size: Some(18.0.into()),
                        spacing: 8.0,
                        side: text_input::Side::Left,
                    })
                    .style(flat_text_input),
            ),
    )
    .width(iced::Length::Fill)
    .center_y(iced::Length::Fill)
    .height(70.0)
    .padding(iced::Padding::new(15.0).left(20.0).right(18.0))
    .style(|theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        shadow: iced::Shadow {
            color: iced::Color::BLACK,
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    })
    .into()
}

fn sidebar<'a>(status: LibraryStatus) -> iced::Element<'a, HomeMessage> {
    let scanning = matches!(status, LibraryStatus::Scanning);

    container(
        column![]
            .padding(5.0)
            .spacing(5.0)
            .push(sidebar_button(0xe02c, "Movies").on_press(HomeMessage::Goto(Tab::Movies)))
            .push(sidebar_button(0xe639, "TV Shows").on_press(HomeMessage::Goto(Tab::TvShows)))
            .push(vertical_space())
            .push(
                sidebar_button(if scanning { 0xe9d0 } else { 0xf3d5 }, "Scan Directories")
                    .on_press_maybe((!scanning).then_some(HomeMessage::ScanDirectories)),
            )
            .push(
                sidebar_button(0xe8b8, "Settings")
                    .on_press_maybe((!scanning).then_some(HomeMessage::OpenSettings)),
            ),
    )
    .width(250.0)
    .height(iced::Length::Fill)
    .style(|theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        ..Default::default()
    })
    .into()
}

fn sidebar_button<'a>(icon: u32, label: &'a str) -> iced::widget::Button<'a, HomeMessage> {
    button(
        row![]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .spacing(10.0)
            .align_y(iced::Alignment::Center)
            .push(
                text(char::from_u32(icon).expect("codepoint"))
                    .font(ICON_FONT)
                    .size(20.0),
            )
            .push(label),
    )
    .width(iced::Length::Fill)
    .height(40.0)
    .padding(iced::Padding::new(5.0).left(10.0))
    .style(clear_button)
}
