use super::{Filter, HomeMessage, Sort, SortDirection};
use crate::{
    library,
    ui::{
        icon, menu_button, rich_checkbox, themed_button, themed_text_input, Tab, HEADER_FONT,
        ICON_FONT,
    },
};
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, pick_list, row, rule, text,
    text_input,
};

pub fn top_bar<'a>(
    search: &str,
    filter: &Filter,
    tab: Tab,
    sort: Sort,
    sort_dir: SortDirection,
    library: &library::Library,
) -> iced::Element<'a, HomeMessage> {
    let show_filters = matches!(tab, Tab::Movies | Tab::TvShows);

    container(
        column![]
            .height(iced::Length::Shrink)
            .push(
                row![]
                    .padding(iced::Padding::new(15.0).right(18.0))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Shrink)
                    .spacing(20.0)
                    .align_y(iced::Alignment::Center)
                    .push(
                        button(
                            icon(0xe5c4)
                                .size(26.0)
                                .width(iced::Length::Fill)
                                .height(40.0)
                                .align_y(iced::Alignment::Center)
                                .align_x(iced::Alignment::Center),
                        )
                        .padding(0.0)
                        .width(40.0)
                        .style(themed_button)
                        .on_press(HomeMessage::Back),
                    )
                    .push(
                        text(match tab {
                            Tab::Home => "Home".into(),
                            Tab::Movies => "Movies".into(),
                            Tab::TvShows => "TV Shows".into(),
                            Tab::TvShow(id) => library.get(id).unwrap().title(),
                            Tab::Season(id) => {
                                let library::Media::Season(season) = library.get(id).unwrap()
                                else {
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
                            .style(themed_text_input),
                    ),
            )
            .push_maybe(show_filters.then(|| {
                horizontal_rule(1.0).style(|theme| rule::Style {
                    color: iced::Color::from_rgb8(40, 40, 40),
                    ..<iced::Theme as rule::Catalog>::default()(theme)
                })
            }))
            .push_maybe(show_filters.then(|| {
                row![]
                    .height(iced::Length::Shrink)
                    .padding(10.0)
                    .spacing(10.0)
                    .push(rich_checkbox(
                        row![]
                            .spacing(5.0)
                            .push(icon(0xe8f4).color(iced::Color::from_rgb8(68, 161, 50)))
                            .push("Watched"),
                        filter.watched,
                        HomeMessage::ToggleFilterWatched,
                    ))
                    .push(rich_checkbox(
                        row![]
                            .spacing(5.0)
                            .push(icon(0xf723).color(iced::Color::from_rgb8(95, 143, 245)))
                            .push("Partally watched"),
                        filter.partially_watched,
                        HomeMessage::ToggleFilterPartiallyWatched,
                    ))
                    .push(rich_checkbox(
                        row![]
                            .spacing(5.0)
                            .push(icon(0xe8f5).color(iced::Color::from_rgb8(200, 200, 200)))
                            .push("Not watched"),
                        filter.not_watched,
                        HomeMessage::ToggleFilterNotWatched,
                    ))
                    .push(horizontal_space())
                    .push(
                        menu_button(
                            row![]
                                .spacing(5.0)
                                .push(icon(0xe152))
                                .push(text(sort.to_string())),
                            || {
                                container(
                                    column![].width(150.0).spacing(5.0).extend(
                                        [
                                            Sort::Name,
                                            Sort::Watched,
                                            Sort::DateAdded,
                                            Sort::LastWatched,
                                        ]
                                        .map(|sort| {
                                            button(text(sort.to_string()))
                                                .width(iced::Length::Fill)
                                                .style(themed_button)
                                                .on_press(HomeMessage::SetSort(sort))
                                                .into()
                                        }),
                                    ),
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
                        .location(menu_button::Location::BottomLeft)
                        .style(themed_button),
                    )
                    .push(
                        button(icon(match sort_dir {
                            SortDirection::Ascending => 0xeacf,
                            SortDirection::Descending => 0xead0,
                        }))
                        .style(themed_button)
                        .on_press(HomeMessage::ToggleSortDirection(match sort_dir {
                            SortDirection::Ascending => SortDirection::Descending,
                            SortDirection::Descending => SortDirection::Ascending,
                        })),
                    )
            })),
    )
    .width(iced::Length::Fill)
    .center_y(iced::Length::Fill)
    .height(iced::Length::Shrink)
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
