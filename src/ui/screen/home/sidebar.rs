use super::{HomeAction, HomeMessage, Tab};
use crate::ui::{clear_button, icon, menu_button, LibraryStatus, ICON_FONT};
use iced::widget::{button, column, container, row, text, vertical_space};

pub fn sidebar<'a>(status: LibraryStatus) -> iced::Element<'a, HomeMessage> {
    let scanning = matches!(status, LibraryStatus::Scanning);

    container(
        column![]
            .padding(5.0)
            .spacing(5.0)
            .push(sidebar_button(0xe88a, "Home").on_press(HomeMessage::Goto(Tab::Home)))
            .push(sidebar_button(0xe02c, "Movies").on_press(HomeMessage::Goto(Tab::Movies)))
            .push(sidebar_button(0xe639, "TV Shows").on_press(HomeMessage::Goto(Tab::TvShows)))
            .push(vertical_space())
            .push(
                row![]
                    .width(iced::Length::Fill)
                    .spacing(5.0)
                    .push(
                        sidebar_button(if scanning { 0xe9d0 } else { 0xf3d5 }, "Scan Directories")
                            .on_press_maybe(
                                (!scanning)
                                    .then_some(HomeMessage::Action(HomeAction::ScanDirectories)),
                            ),
                    )
                    .push(
                        menu_button(
                            container(icon(0xe5d2).size(20.0)).center_y(iced::Length::Fill),
                            move || {
                                container(
                                    column![]
                                        .width(200.0)
                                        .spacing(5.0)
                                        .push(
                                            sidebar_button(0xe760, "Purge").on_press_maybe(
                                                (!scanning).then_some(HomeMessage::Action(
                                                    HomeAction::Purge,
                                                )),
                                            ),
                                        )
                                        .push(sidebar_button(0xe627, "Force Scan").on_press_maybe(
                                            (!scanning).then_some(HomeMessage::Action(
                                                HomeAction::ForceScan,
                                            )),
                                        )),
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
                        .height(40.0)
                        .style(clear_button),
                    ),
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
                    .size(19.0),
            )
            .push(label),
    )
    .width(iced::Length::Fill)
    .height(40.0)
    .padding(iced::Padding::new(5.0).left(10.0))
    .style(clear_button)
}
