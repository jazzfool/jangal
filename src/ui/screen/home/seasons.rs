use super::{
    media_menu, poster_image, search_episode, search_maybe, search_season, watched_icon,
    HomeMessage,
};
use crate::{
    library,
    ui::{clear_button, icon, HEADER_FONT},
};
use iced::widget::{button, column, container, horizontal_space, hover, row, text};
use itertools::Itertools;
use std::path::PathBuf;

pub fn season_list<'a>(
    search: Option<&str>,
    id: library::MediaId,
    _series: &library::Series,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    column![]
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .padding(iced::Padding::ZERO.top(20.0).bottom(20.0))
        .spacing(20.0)
        .extend(
            search_maybe(
                library::find_seasons(id, library),
                search.map(|search| {
                    |(id, season): &(&library::MediaId, &library::Season)| {
                        search_season(search, **id, season, library)
                    }
                }),
                |(_, a), (_, b)| a.metadata.season.cmp(&b.metadata.season),
            )
            .map(|(id, season)| season_panel(search, *id, season, library)),
        )
        .into()
}

pub fn season_panel<'a>(
    search: Option<&str>,
    id: library::MediaId,
    season: &library::Season,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    column![]
        .width(iced::Length::Fill)
        .height(iced::Length::Shrink)
        .spacing(10.0)
        .max_width(800.0)
        .push(
            row![]
                .spacing(20.0)
                .push(poster_image(
                    season.metadata.poster.as_ref().map(PathBuf::as_path),
                ))
                .push(
                    column![]
                        .push(
                            row![]
                                .padding(iced::Padding::ZERO.right(10.0))
                                .push(
                                    column![]
                                        .push(
                                            text(format!("S{:02}", season.metadata.season))
                                                .size(14.0)
                                                .style(|theme: &iced::Theme| text::Style {
                                                    color: Some(
                                                        theme
                                                            .extended_palette()
                                                            .background
                                                            .strong
                                                            .color,
                                                    ),
                                                    ..Default::default()
                                                }),
                                        )
                                        .push(
                                            text(season.metadata.title.clone())
                                                .size(24.0)
                                                .font(HEADER_FONT)
                                                .line_height(1.5),
                                        ),
                                )
                                .push(horizontal_space())
                                .push(media_menu(id, library)),
                        )
                        .push(
                            text(season.metadata.overview.clone().unwrap_or_default()).style(
                                |theme: &iced::Theme| text::Style {
                                    color: Some(theme.palette().text.scale_alpha(0.8)),
                                    ..Default::default()
                                },
                            ),
                        ),
                ),
        )
        .push(episode_list(search, id, season, library))
        .into()
}

fn episode_list<'a>(
    search: Option<&str>,
    id: library::MediaId,
    _season: &library::Season,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    let mut episodes = search_maybe(
        library::find_episodes(id, library),
        search.map(|search| {
            |(id, episode): &(&library::MediaId, &library::Episode)| {
                search_episode(search, **id, episode)
            }
        }),
        |(_, a), (_, b)| a.metadata.episode.cmp(&b.metadata.episode),
    )
    .collect_vec();

    if episodes.is_empty() {
        episodes = library::find_episodes(id, library).collect_vec();
    }

    column![]
        .width(iced::Length::Fill)
        .spacing(5.0)
        .extend(
            episodes
                .into_iter()
                .map(|(id, episode)| episode_entry(*id, episode, library)),
        )
        .into()
}

fn episode_entry<'a>(
    id: library::MediaId,
    episode: &library::Episode,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    hover(
        button(
            row![]
                .spacing(10.0)
                .align_y(iced::Alignment::Center)
                .padding(iced::Padding::new(0.0).left(30.0))
                .push(
                    text(format!("E{:02}", episode.metadata.episode))
                        .size(14.0)
                        .width(30.0)
                        .style(|theme: &iced::Theme| text::Style {
                            color: Some(theme.extended_palette().background.strong.color),
                            ..Default::default()
                        }),
                )
                .push(text(episode.metadata.title.clone()))
                .push(horizontal_space())
                .push(container(watched_icon(episode.video.watched, false)).style(
                    |theme: &iced::Theme| container::Style {
                        text_color: Some(theme.extended_palette().background.strong.color),
                        ..Default::default()
                    },
                ))
                .push(media_menu(id, library)),
        )
        .width(iced::Length::Fill)
        .style(clear_button)
        .on_press(HomeMessage::Play(id)),
        icon(0xe037)
            .size(26.0)
            .width(40.0)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center),
    )
    .into()
}
