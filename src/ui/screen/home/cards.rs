use super::{menu_item, poster_image, search_maybe, watched_icon, HomeMessage, Tab};
use crate::{
    library,
    ui::{clear_button, icon, menu_button},
};
use iced::widget::{column, container, horizontal_space, hover, mouse_area, row, stack, text};

pub fn card_grid<'a, 'b>(
    search: Option<&str>,
    media: impl Iterator<Item = (&'b library::MediaId, &'b library::Media)>,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    row![]
        .spacing(10.0)
        .padding(iced::Padding::new(0.0).top(20.0).bottom(20.0))
        .clip(true)
        .extend(
            search_maybe(
                media,
                search.map(|search| {
                    |(_id, media): &(&library::MediaId, &library::Media)| {
                        sublime_fuzzy::best_match(search, &media.full_title().unwrap_or_default())
                            .map(|m| m.score())
                    }
                }),
                |(_, a), (_, b)| a.full_title().cmp(&b.full_title()),
            )
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
                text(media.full_title().unwrap_or(String::from("Unknown Media")))
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
                        .push(horizontal_space())
                        .push(
                            menu_button(
                                container(icon(0xe5d2).size(20.0)).center_y(iced::Length::Fill),
                                || {
                                    container(
                                        column![]
                                            .push(menu_item(0xe8f5, "Mark unwatched"))
                                            .push(menu_item(0xe8f4, "Mark watched"))
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
                            .padding(iced::Padding::new(0.0).left(5.0).right(5.0))
                            .style(clear_button),
                        ),
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
            .push_maybe(media.path().map(|_| {
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
