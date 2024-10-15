mod cards;
mod seasons;
mod sidebar;

use super::Screen;
use crate::{
    library,
    ui::{
        clear_button, clear_scrollable, flat_text_input, icon, menu_button, AppState, Tab,
        HEADER_FONT, ICON_FONT,
    },
};
use iced::widget::{
    button, center, column, container, horizontal_rule, horizontal_space, image, row, rule,
    scrollable, stack, text, text_input, vertical_rule,
};
use itertools::Itertools;
use std::{path::Path, time::Duration};

pub struct Home {
    search: String,
    save_task: Option<iced::task::Handle>, // to debounce saves
}

impl Home {
    pub fn new() -> (Self, iced::Task<HomeMessage>) {
        (
            Home {
                search: String::new(),
                save_task: None,
            },
            iced::Task::none(),
        )
    }

    fn save(&mut self, state: &mut AppState) -> iced::Task<HomeMessage> {
        if let Some(task) = self.save_task.take() {
            task.abort();
        }
        let fut = state.save_library();
        let (task, handle) = iced::Task::perform(
            async move {
                async_std::task::sleep(Duration::from_secs(10)).await;
                fut.await.unwrap();
            },
            |_| (),
        )
        .discard()
        .abortable();
        self.save_task = Some(handle);
        task
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
            HomeMessage::Goto(tab) => {
                let last_tab = state.tab_stack.back_mut().unwrap();
                if last_tab.overwrites(tab) {
                    *last_tab = tab;
                } else {
                    state.tab_stack.push_back(tab);
                }
                iced::Task::none()
            }
            HomeMessage::Back => {
                state.tab_stack.pop_back();
                if state.tab_stack.is_empty() {
                    state.tab_stack.push_back(Tab::Movies);
                }
                iced::Task::none()
            }
            HomeMessage::MarkUnwatched(id) => {
                library::set_watched(id, library::Watched::No, &mut state.library);
                self.save(state)
            }
            HomeMessage::MarkWatched(id) => {
                library::set_watched(id, library::Watched::Yes, &mut state.library);
                self.save(state)
            }
            _ => iced::Task::none(),
        }
    }

    fn view(&self, state: &AppState) -> iced::Element<HomeMessage> {
        let search = (!self.search.trim().is_empty()).then_some(self.search.as_str());
        let tab = state.tab_stack.back().copied().unwrap();

        row![]
            .push(sidebar::sidebar(state.library_status))
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
                                center(match tab {
                                    Tab::Home => column![]
                                        .width(iced::Length::Fill)
                                        .padding(iced::Padding::new(40.0).top(20.0).bottom(20.0))
                                        .spacing(10.0)
                                        .push(text("Keep watching").font(HEADER_FONT).size(24.0))
                                        .push(cards::card_grid(
                                            search,
                                            state.library.iter().filter(|(_, media)| {
                                                media.video().is_some_and(|video| {
                                                    matches!(
                                                        video.watched,
                                                        library::Watched::Partial { .. }
                                                    )
                                                })
                                            }),
                                            &state.library,
                                            |&(a_id, a), &(b_id, b), _| {
                                                library::last_watched(*b_id, b, &state.library).cmp(
                                                    &library::last_watched(
                                                        *a_id,
                                                        a,
                                                        &state.library,
                                                    ),
                                                )
                                            },
                                            Some(20),
                                        ))
                                        .push(text("Recently added").font(HEADER_FONT).size(24.0))
                                        .push(cards::card_grid(
                                            search,
                                            state.library.iter().filter(|(_, media)| {
                                                media.video().is_some_and(|video| {
                                                    (chrono::Local::now() - video.added).num_days()
                                                        < 7
                                                })
                                            }),
                                            &state.library,
                                            |&(_a_id, a), &(_b_id, b), _| {
                                                b.video()
                                                    .unwrap()
                                                    .added
                                                    .cmp(&a.video().unwrap().added)
                                            },
                                            Some(20),
                                        ))
                                        .into(),
                                    Tab::Movies | Tab::TvShows => cards::card_grid(
                                        search,
                                        state.library.iter().filter(|(_, media)| match tab {
                                            Tab::Movies => {
                                                matches!(media, library::Media::Movie(_))
                                            }
                                            Tab::TvShows => {
                                                matches!(media, library::Media::Series(_))
                                            }
                                            _ => unreachable!(),
                                        }),
                                        &state.library,
                                        cards::sort_alphanumeric,
                                        None,
                                    ),
                                    Tab::TvShow(id) => seasons::season_list(
                                        search,
                                        id,
                                        state
                                            .library
                                            .get(id)
                                            .and_then(|media| match media {
                                                library::Media::Series(series) => Some(series),
                                                _ => None,
                                            })
                                            .unwrap(),
                                        &state.library,
                                    ),
                                    Tab::Season(id) => seasons::season_panel(
                                        search,
                                        id,
                                        state
                                            .library
                                            .get(id)
                                            .and_then(|media| match media {
                                                library::Media::Season(season) => Some(season),
                                                _ => None,
                                            })
                                            .unwrap(),
                                        &state.library,
                                    ),
                                })
                                .height(iced::Length::Shrink)
                                .align_y(iced::Alignment::Start)
                                .padding(iced::Padding::ZERO.left(20.0).right(20.0)),
                            )
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .style(clear_scrollable)
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
                            .push(top_bar(&self.search, tab, &state.library))
                            .push(horizontal_rule(1.0).style(|theme| rule::Style {
                                color: iced::Color::from_rgb8(40, 40, 40),
                                ..<iced::Theme as rule::Catalog>::default()(theme)
                            })),
                    ),
            )
            .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HomeAction {
    ScanDirectories,
    Purge,
    ForceScan,
}

#[derive(Debug, Clone)]
pub enum HomeMessage {
    Play(library::MediaId),
    OpenSettings,
    Search(String),
    Action(HomeAction),
    Goto(Tab),
    Back,
    MarkWatched(library::MediaId),
    MarkUnwatched(library::MediaId),
}

fn poster_image<'a>(poster: Option<&Path>) -> iced::Element<'a, HomeMessage> {
    let poster = poster.map(Path::to_path_buf);

    container(if let Some(img) = &poster {
        image(img)
            .content_fit(iced::ContentFit::Cover)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    } else {
        iced::Element::from("")
    })
    .width(150.0)
    .height(225.0)
    .clip(true)
    .style(move |_| container::Style {
        background: poster
            .is_none()
            .then_some(iced::Background::Color(iced::Color::WHITE)),
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
    })
    .into()
}

fn watched_icon<'a>(watched: library::Watched, icon_first: bool) -> iced::Element<'a, HomeMessage> {
    let (color, codepoint) = match watched {
        library::Watched::No => (iced::Color::from_rgb8(200, 200, 200), 0xe8f5),
        library::Watched::Partial { percent, .. } => (iced::Color::from_rgb8(95, 143, 245), {
            if percent < 0.125 {
                0xf726
            } else if percent < 0.25 {
                0xf725
            } else if percent < 0.5 {
                0xf724
            } else if percent < 0.625 {
                0xf723
            } else if percent < 0.75 {
                0xf722
            } else {
                0xf721
            }
        }),
        library::Watched::Yes => (iced::Color::from_rgb8(68, 161, 50), 0xe8f4),
    };
    let icon = icon(codepoint).color(color);

    let label = match watched {
        library::Watched::Partial { percent, .. } => format!("{}%", (percent * 100.0) as u8),
        _ => String::new(),
    };

    let row = row![].align_y(iced::Alignment::Center).spacing(5.0);
    let row = if icon_first {
        row.push(icon).push(text(label).color(color))
    } else {
        row.push(text(label).color(color)).push(icon)
    };

    row.into()
}

fn search_episode(
    search: &str,
    _id: library::MediaId,
    episode: &library::Episode,
) -> Option<isize> {
    [
        format!(
            "S{:02}E{:02}",
            episode.metadata.season, episode.metadata.episode
        ),
        episode.metadata.title.clone(),
    ]
    .into_iter()
    .filter_map(|s| sublime_fuzzy::best_match(search, &s).map(|m| m.score()))
    .max()
}

fn search_season(
    search: &str,
    id: library::MediaId,
    season: &library::Season,
    library: &library::Library,
) -> Option<isize> {
    [
        format!("S{:02}", season.metadata.season),
        season.metadata.title.clone(),
        season.metadata.overview.clone().unwrap_or_default(),
    ]
    .into_iter()
    .filter_map(|s| sublime_fuzzy::best_match(search, &s).map(|m| m.score()))
    .chain(
        library::find_episodes(id, library)
            .flat_map(|(id, episode)| search_episode(search, *id, episode)),
    )
    .max()
}

fn search_maybe<T>(
    iter: impl Iterator<Item = T>,
    search: Option<impl Fn(&T) -> Option<isize>>,
    sort: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> impl Iterator<Item = T> {
    iter.filter_map(|x| {
        if let Some(search) = &search {
            Some((search(&x)?, x))
        } else {
            Some((0, x))
        }
    })
    .sorted_by(|(a_score, a), (b_score, b)| b_score.cmp(a_score).then_with(|| sort(a, b)))
    .map(|(_, x)| x)
}

fn media_menu<'a>(
    id: library::MediaId,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    let watched = library::calculate_watched(id, library).unwrap_or(library::Watched::No);

    menu_button(
        container(icon(0xe5d2).size(20.0)).center(iced::Length::Fill),
        move || {
            container(
                column![]
                    .push_maybe(
                        matches!(
                            watched,
                            library::Watched::Partial { .. } | library::Watched::Yes
                        )
                        .then(|| {
                            menu_item(0xe8f5, "Mark unwatched")
                                .on_press(HomeMessage::MarkUnwatched(id))
                        }),
                    )
                    .push_maybe(
                        matches!(
                            watched,
                            library::Watched::No | library::Watched::Partial { .. }
                        )
                        .then(|| {
                            menu_item(0xe8f4, "Mark watched").on_press(HomeMessage::MarkWatched(id))
                        }),
                    )
                    .width(200.0)
                    .spacing(5.0),
            )
            .padding(5.0)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(
                    theme.extended_palette().background.strong.text,
                )),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: iced::border::radius(10.0),
                },
                shadow: iced::Shadow {
                    color: iced::Color::BLACK.scale_alpha(1.2),
                    offset: iced::Vector::new(0.0, 3.0),
                    blur_radius: 20.0,
                },
                ..Default::default()
            })
            .into()
        },
    )
    .padding(0.0)
    .width(30.0)
    .height(30.0)
    .style(clear_button)
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
            .spacing(20.0)
            .align_y(iced::Alignment::Center)
            .push(
                button(
                    icon(0xe5c4)
                        .size(26.0)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .align_y(iced::Alignment::Center)
                        .align_x(iced::Alignment::Center),
                )
                .padding(0.0)
                .width(40.0)
                .style(clear_button)
                .on_press(HomeMessage::Back),
            )
            .push(
                text(match tab {
                    Tab::Home => "Home".into(),
                    Tab::Movies => "Movies".into(),
                    Tab::TvShows => "TV Shows".into(),
                    Tab::TvShow(id) => library.get(id).unwrap().title(),
                    Tab::Season(id) => {
                        let library::Media::Season(season) = library.get(id).unwrap() else {
                            panic!()
                        };
                        let series = library.get(season.series).unwrap();
                        format!(
                            "{} S{:02} - {}",
                            series.title(),
                            season.metadata.season,
                            season.metadata.title
                        )
                    }
                })
                .font(HEADER_FONT)
                .size(28.0)
                .color(iced::Color::from_rgba8(210, 210, 210, 1.0)),
            )
            .push(horizontal_space())
            .push(
                text_input("Search", search)
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
    .padding(iced::Padding::new(15.0).right(18.0))
    .style(|theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        shadow: iced::Shadow {
            color: iced::Color::BLACK.scale_alpha(1.5),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    })
    .into()
}

fn menu_item<'a>(icon: u32, label: &'a str) -> iced::widget::Button<'a, HomeMessage> {
    button(
        row![]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .spacing(10.0)
            .align_y(iced::Alignment::Center)
            .push(
                text(char::from_u32(icon).expect("codepoint"))
                    .font(ICON_FONT)
                    .size(16.0),
            )
            .push(label),
    )
    .width(iced::Length::Fill)
    .height(30.0)
    .padding(iced::Padding::new(5.0).left(10.0))
    .style(clear_button)
}
