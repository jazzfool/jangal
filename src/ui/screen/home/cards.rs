use super::{
    HomeMessage, Tab, collection_menu, media_menu, poster_image, search_maybe, watched_icon,
};
use crate::{library, ui::icon};
use iced::widget::{column, container, hover, mouse_area, row, space, stack, text};

pub fn card_grid<'a, 'b>(
    search: Option<&str>,
    media: impl Iterator<Item = (&'b library::MediaId, &'b library::Media)>,
    library: &library::Library,
    sort: impl Fn(
        &(&library::MediaId, &library::Media),
        &(&library::MediaId, &library::Media),
        &library::Library,
    ) -> std::cmp::Ordering,
    limit: Option<usize>,
) -> iced::Element<'a, HomeMessage> {
    row![]
        .spacing(10.0)
        .padding(iced::Padding::new(0.0).top(20.0).bottom(20.0))
        .clip(true)
        .extend(
            search_maybe(
                media,
                search.map(|search| {
                    |&(id, _media): &(&library::MediaId, &library::Media)| {
                        sublime_fuzzy::best_match(search, &library::full_title(*id, library))
                            .map(|m| m.score())
                    }
                }),
                |a, b| sort(a, b, library),
            )
            .take(limit.unwrap_or(usize::MAX))
            .map(|(id, media)| media_card(*id, media, library)),
        )
        .wrap()
        .into()
}

fn media_card<'a>(
    id: library::MediaId,
    media: &library::Media,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    let poster = match media {
        library::Media::Episode(episode) => library.get(episode.season).unwrap().poster(),
        _ => media.poster(),
    };

    mouse_area(hover(
        column![]
            .spacing(5.0)
            .width(150.0)
            .clip(true)
            .push(poster_image(poster))
            .push(
                text(library::full_title(id, library))
                    .wrapping(text::Wrapping::None)
                    .size(14.0),
            ),
        stack![]
            .width(150.0)
            .height(225.0)
            .push(
                container(
                    row![]
                        .spacing(5.0)
                        .width(iced::Length::Fill)
                        .height(30.0)
                        .align_y(iced::Alignment::Center)
                        .push(watched_icon(
                            library::calculate_watched(id, library).unwrap(),
                            true,
                        ))
                        .push(space::horizontal())
                        .push(collection_menu(id, library))
                        .push(media_menu(id, library)),
                )
                .width(iced::Length::Fill)
                .height(150.0)
                .padding(iced::Padding::new(5.0).left(10.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                        iced::gradient::Linear::new(0.0)
                            .add_stop(0.0, iced::Color::BLACK.scale_alpha(0.0))
                            .add_stop(1.0, iced::Color::BLACK.scale_alpha(0.95)),
                    ))),
                    ..Default::default()
                }),
            )
            .push(media.video().map(|_| {
                icon(0xe037)
                    .size(36.0)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center)
            })),
    ))
    .interaction(iced::mouse::Interaction::Pointer)
    .on_press(match media {
        library::Media::Uncategorised(_)
        | library::Media::Movie(_)
        | library::Media::Episode(_) => HomeMessage::Play(id),
        library::Media::Series(_) => HomeMessage::Goto(Tab::TvShow(id)),
        library::Media::Season(_) => HomeMessage::Goto(Tab::Season(id)),
    })
    .into()
}
