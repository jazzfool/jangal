mod cards;
mod seasons;
mod sidebar;
mod top_bar;

use super::Screen;
use crate::{
    library,
    ui::{
        AppState, HEADER_FONT, ICON_FONT, Tab, find_focused_maybe, icon, menu_button, open_path,
        themed_button, themed_menu, themed_scrollable,
    },
};
use iced::widget::{
    button, center, column, container, image, opaque, row, rule, scrollable, space, stack, text,
};
use itertools::Itertools;
use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};
use top_bar::top_bar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filter {
    pub watched: bool,
    pub partially_watched: bool,
    pub not_watched: bool,
}

impl Filter {
    pub fn filter(&self, id: library::MediaId, library: &library::Library) -> bool {
        let watched = library::calculate_watched(id, library);
        match watched {
            Some(library::Watched::No) => self.not_watched,
            Some(library::Watched::Partial { .. }) => self.partially_watched,
            Some(library::Watched::Yes) => self.watched,
            None => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Watched,
    DateAdded,
    LastWatched,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Sort::Name => "Name",
            Sort::Watched => "Watched",
            Sort::DateAdded => "Date Added",
            Sort::LastWatched => "Last Watched",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

pub struct Home {
    search: String,
    filter: Filter,
    sort: Sort,
    sort_dir: SortDirection,

    save_task: Option<iced::task::Handle>, // to debounce saves
    sidebar_action: sidebar::Action,
}

impl Home {
    pub fn new() -> (Self, iced::Task<HomeMessage>) {
        (
            Home {
                search: String::new(),
                filter: Filter {
                    watched: true,
                    partially_watched: true,
                    not_watched: true,
                },
                sort: Sort::Name,
                sort_dir: SortDirection::Ascending,

                save_task: None,
                sidebar_action: sidebar::Action::None,
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
                if last_tab.overwrites(&tab) {
                    *last_tab = tab;
                } else {
                    state.tab_stack.push_back(tab);
                }
                iced::Task::none()
            }
            HomeMessage::Back => {
                state.tab_stack.pop_back();
                if state.tab_stack.is_empty() {
                    state.tab_stack.push_back(Tab::Home);
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
            HomeMessage::OpenDirectory(path) => {
                open_path(&path);
                iced::Task::none()
            }
            HomeMessage::ToggleFilterWatched(toggle) => {
                self.filter.watched = toggle;
                iced::Task::none()
            }
            HomeMessage::ToggleFilterPartiallyWatched(toggle) => {
                self.filter.partially_watched = toggle;
                iced::Task::none()
            }
            HomeMessage::ToggleFilterNotWatched(toggle) => {
                self.filter.not_watched = toggle;
                iced::Task::none()
            }
            HomeMessage::SetSort(sort) => {
                self.sort = sort;
                iced::Task::none()
            }
            HomeMessage::ToggleSortDirection(sort_dir) => {
                self.sort_dir = sort_dir;
                iced::Task::none()
            }
            HomeMessage::NewCollection => {
                state.library.insert_collection();
                iced::Task::none()
            }
            HomeMessage::BeginRenameCollection(id) => {
                if let Some(collection) = state.library.collection(id) {
                    self.sidebar_action = sidebar::Action::RenameCollection {
                        id,
                        name: collection.name().into(),
                    };
                } else {
                    self.sidebar_action = sidebar::Action::None;
                }
                iced::advanced::widget::operate(
                    iced::advanced::widget::operation::focusable::focus(
                        iced::advanced::widget::Id::new("sidebar_collection_name"),
                    ),
                )
            }
            HomeMessage::BeginDeleteCollection(id) => {
                self.sidebar_action = sidebar::Action::DeleteCollection(id);
                iced::Task::none()
            }
            HomeMessage::RenameCollection(id) => {
                let sidebar::Action::RenameCollection {
                    id: action_id,
                    name,
                } = &self.sidebar_action
                else {
                    self.sidebar_action = sidebar::Action::None;
                    return iced::Task::none();
                };

                if let Some(collection) = state.library.collection_mut(id) {
                    if *action_id == id {
                        collection.set_name(name);
                    }
                }

                self.sidebar_action = sidebar::Action::None;

                iced::Task::none()
            }
            HomeMessage::DeleteCollection(id) => {
                if self.sidebar_action != sidebar::Action::DeleteCollection(id) {
                    self.sidebar_action = sidebar::Action::None;
                    return iced::Task::none();
                }
                self.sidebar_action = sidebar::Action::None;

                state.tab_stack.retain(|tab| match tab {
                    Tab::Collection(cid) if *cid == id => false,
                    _ => true,
                });
                if state.tab_stack.is_empty() {
                    state.tab_stack.push_back(Tab::Home);
                }

                state.library.remove_collection(id);
                iced::Task::none()
            }
            HomeMessage::RenameCollectionInput(new_name) => {
                let sidebar::Action::RenameCollection { name, .. } = &mut self.sidebar_action
                else {
                    return iced::Task::none();
                };
                *name = new_name;
                iced::Task::none()
            }
            HomeMessage::CancelSidebarAction => {
                self.sidebar_action = sidebar::Action::None;
                iced::Task::none()
            }
            HomeMessage::CheckCollectionInputFocus => {
                iced::advanced::widget::operate(find_focused_maybe()).map(|id| {
                    if id != Some(iced::advanced::widget::Id::new("sidebar_collection_name")) {
                        HomeMessage::CancelSidebarAction
                    } else {
                        HomeMessage::None
                    }
                })
            }
            HomeMessage::ToggleMediaCollection(media_id, collection_id) => {
                let Some(collection) = state.library.collection_mut(collection_id) else {
                    return iced::Task::none();
                };

                if collection.contains(media_id) {
                    collection.remove(media_id);
                } else {
                    collection.insert(media_id);
                }

                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view<'a, 'b>(&'a self, state: &'a AppState) -> iced::Element<'b, HomeMessage>
    where
        'a: 'b,
    {
        let search = (!self.search.trim().is_empty()).then_some(self.search.as_str());
        let tab = state.tab_stack.back().cloned().unwrap();

        row![]
            .push(sidebar::sidebar(state.library_status, state.library.iter_collections(), self.sidebar_action.clone()))
            .push(rule::vertical(1.0).style(|theme| rule::Style {
                color: iced::Color::from_rgb8(40, 40, 40),
                ..<iced::Theme as rule::Catalog>::default()(theme)
            }))
            .push(
                stack![]
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .clip(true)
                    .push(
                        container(
                            scrollable(
                                center(match tab.clone() {
                                    Tab::Home => column![]
                                        .width(iced::Length::Fill)
                                        .padding(iced::Padding::new(40.0).top(20.0).bottom(20.0))
                                        .spacing(10.0)
                                        .push(
                                            row![]
                                                .spacing(30)
                                                .align_y(iced::alignment::Vertical::Center)
                                                .push(text("Keep Watching").font(HEADER_FONT).size(24.0))
                                                .push(rule::horizontal(1.0).style(|theme| rule::Style {
                                                    color: iced::Color::from_rgb8(40, 40, 40),
                                                    ..<iced::Theme as rule::Catalog>::default()(theme)
                                                }))
                                        )
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
                                            |&(a_id, _), &(b_id, _), _| {
                                                library::last_watched(*b_id, &state.library).cmp(
                                                    &library::last_watched(*a_id, &state.library),
                                                )
                                            },
                                            Some(20),
                                        ))
                                        .push(
                                            row![]
                                                .spacing(30)
                                                .align_y(iced::alignment::Vertical::Center)
                                                .push(text("Recently Added").font(HEADER_FONT).size(24.0))
                                                .push(rule::horizontal(1.0).style(|theme| rule::Style {
                                                    color: iced::Color::from_rgb8(40, 40, 40),
                                                    ..<iced::Theme as rule::Catalog>::default()(theme)
                                                }))
                                        )
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
                                        state.library.iter().filter(|(id, media)| match tab {
                                            Tab::Movies => {
                                                matches!(media, library::Media::Movie(_))
                                            }
                                            Tab::TvShows => {
                                                matches!(media, library::Media::Series(_))
                                            }
                                            _ => unreachable!(),
                                        } && self.filter.filter(**id, &state.library)),
                                        &state.library,
                                        |a, b, library| {
                                            sort_by(a, b, library, self.sort, self.sort_dir)
                                        },
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
                                    Tab::Collection(name) => {
                                        if let Some(iter) = state.library.collection_iter(&name) {
                                            cards::card_grid(
                                                search,
                                                iter.filter(|(id, _)| self.filter.filter(**id, &state.library)),
                                                &state.library,
                                                |a, b, library| {
                                                    sort_by(a, b, library, self.sort, self.sort_dir)
                                                },
                                                None,
                                            )
                                        } else {
                                            space().into()
                                        }
                                    },
                                })
                                .height(iced::Length::Shrink)
                                .align_y(iced::Alignment::Start)
                                .padding(iced::Padding::ZERO.left(20.0).right(20.0)),
                            )
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .style(themed_scrollable)
                            .direction(
                                scrollable::Direction::Vertical(scrollable::Scrollbar::new()),
                            ),
                        )
                        .clip(true)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .padding(iced::Padding::ZERO.top(if matches!(tab, Tab::Movies | Tab::TvShows | Tab::Collection(_)) { 115.0 } else { 70.0 })),
                    )
                    .push(
                        column![]
                            .width(iced::Length::Fill)
                            .height(iced::Length::Shrink)
                            .push(top_bar(
                                &self.search,
                                &self.filter,
                                tab,
                                self.sort,
                                self.sort_dir,
                                &state.library,
                            ))
                            .push(rule::horizontal(1.0).style(|theme| rule::Style {
                                color: iced::Color::from_rgb8(40, 40, 40),
                                ..<iced::Theme as rule::Catalog>::default()(theme)
                            })),
                    ),
            )
            .into()
    }

    fn subscription(&self) -> iced::Subscription<HomeMessage> {
        if let sidebar::Action::RenameCollection { .. } = self.sidebar_action {
            iced::event::listen().map(|_| HomeMessage::CheckCollectionInputFocus)
        } else {
            iced::Subscription::none()
        }
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
    OpenDirectory(PathBuf),
    ToggleFilterWatched(bool),
    ToggleFilterPartiallyWatched(bool),
    ToggleFilterNotWatched(bool),
    SetSort(Sort),
    ToggleSortDirection(SortDirection),
    ToggleMediaCollection(library::MediaId, library::CollectionId),

    NewCollection,
    BeginRenameCollection(library::CollectionId),
    BeginDeleteCollection(library::CollectionId),
    RenameCollection(library::CollectionId),
    DeleteCollection(library::CollectionId),
    RenameCollectionInput(String),
    CancelSidebarAction,
    CheckCollectionInputFocus,

    None,
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
    .padding(1.0)
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

fn media_menu<'a, 'b>(
    id: library::MediaId,
    library: &'b library::Library,
) -> iced::Element<'a, HomeMessage> {
    let media = library.get(id).unwrap();
    let path = media
        .video()
        .map(|video| video.path.parent().unwrap().to_path_buf());
    let watched = library::calculate_watched(id, library).unwrap_or(library::Watched::No);

    menu_button(
        container(icon(0xe5d2).size(20.0)).center(iced::Length::Fill),
        opaque(
            container(
                column![]
                    .push(path.clone().map(|path| {
                        menu_item(0xe89e, "Open file directory")
                            .on_press(HomeMessage::OpenDirectory(path))
                    }))
                    .push(
                        matches!(
                            watched,
                            library::Watched::Partial { .. } | library::Watched::Yes
                        )
                        .then(|| {
                            menu_item(0xe8f5, "Mark unwatched")
                                .on_press(HomeMessage::MarkUnwatched(id))
                        }),
                    )
                    .push(
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
            .style(themed_menu),
        ),
    )
    .padding(0.0)
    .width(30.0)
    .height(30.0)
    .style(themed_button)
    .into()
}

fn collection_menu<'a, 'b>(
    id: library::MediaId,
    library: &'b library::Library,
) -> iced::Element<'a, HomeMessage> {
    let collections: Vec<_> = library
        .iter_collections()
        .map(|(collection_id, collection)| {
            (
                *collection_id,
                collection.name().to_owned(),
                collection.contains(id),
            )
        })
        .collect();

    menu_button(
        container(icon(0xe02e).size(20.0)).center(iced::Length::Fill),
        opaque(
            container(
                column![]
                    .width(200.0)
                    .spacing(5.0)
                    .extend(
                        collections
                            .iter()
                            .map(|(collection_id, name, in_collection)| {
                                menu_item(if *in_collection { 0xe834 } else { 0xe835 }, name)
                                    .on_press(HomeMessage::ToggleMediaCollection(
                                        id,
                                        *collection_id,
                                    ))
                                    .into()
                            }),
                    ),
            )
            .padding(5.0)
            .style(themed_menu),
        ),
    )
    .auto_close(false)
    .padding(0.0)
    .width(30.0)
    .height(30.0)
    .style(themed_button)
    .into()
}

pub fn menu_item<'a>(icon: u32, label: impl Into<String>) -> iced::widget::Button<'a, HomeMessage> {
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
            .push(text(label.into())),
    )
    .width(iced::Length::Fill)
    .height(30.0)
    .padding(iced::Padding::new(5.0).left(10.0))
    .style(themed_button)
}

fn sort_by(
    a: &(&library::MediaId, &library::Media),
    b: &(&library::MediaId, &library::Media),
    library: &library::Library,
    sort: Sort,
    direction: SortDirection,
) -> std::cmp::Ordering {
    let a = *a.0;
    let b = *b.0;

    let ord = match sort {
        Sort::Name => library::full_title(a, library).cmp(&library::full_title(b, library)),
        Sort::Watched => library::calculate_watched(a, library)
            .map(|x| x.percent())
            .unwrap_or(0.0)
            .partial_cmp(
                &library::calculate_watched(b, library)
                    .map(|x| x.percent())
                    .unwrap_or(0.0),
            )
            .unwrap_or(std::cmp::Ordering::Equal),
        Sort::DateAdded => library::date_added(a, library).cmp(&library::date_added(b, library)),
        Sort::LastWatched => {
            library::last_watched(a, library).cmp(&library::last_watched(b, library))
        }
    };

    match direction {
        SortDirection::Ascending => ord,
        SortDirection::Descending => ord.reverse(),
    }
}
