use super::{menu_item, HomeAction, HomeMessage, Tab};
use crate::{
    library,
    ui::{
        icon, menu_button, themed_button, themed_text_input, truncate_text, LibraryStatus,
        ICON_FONT,
    },
};
use iced::widget::{
    button, column, container, horizontal_space, hover, row, text, text_input, vertical_space,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    RenameCollection {
        id: library::CollectionId,
        name: String,
    },
    DeleteCollection(library::CollectionId),
}

pub fn sidebar<'a>(
    status: LibraryStatus,
    collections: impl Iterator<Item = (&'a library::CollectionId, &'a library::Collection)>,
    action: Action,
) -> iced::Element<'a, HomeMessage> {
    let scanning = matches!(status, LibraryStatus::Scanning);

    container(
        column![]
            .padding(5.0)
            .spacing(5.0)
            .push(sidebar_button(0xe88a, "Home").on_press(HomeMessage::Goto(Tab::Home)))
            .push(sidebar_button(0xe02c, "Movies").on_press(HomeMessage::Goto(Tab::Movies)))
            .push(sidebar_button(0xe639, "TV Shows").on_press(HomeMessage::Goto(Tab::TvShows)))
            .extend(collections.map(|(id, collection)| {
                let id = *id;

                match &action {
                    Action::RenameCollection {
                        id: action_id,
                        name,
                    } if action_id == &id => sidebar_button(
                        0xe04a,
                        text_input("Name", &name)
                            .id("sidebar_collection_name")
                            .style(themed_text_input)
                            .on_input(HomeMessage::RenameCollectionInput)
                            .on_submit(HomeMessage::RenameCollection(id)),
                    )
                    .into(),
                    Action::DeleteCollection(action_id) if action_id == &id => sidebar_button(
                        0xe04a,
                        row![]
                            .spacing(5.0)
                            .height(iced::Length::Fill)
                            .align_y(iced::Alignment::Center)
                            .push(text(truncate_text(collection.name(), 18)))
                            .push(horizontal_space())
                            .push(
                                button(
                                    container(
                                        icon(0xe872)
                                            .size(20.0)
                                            .color(iced::Color::from_rgb8(237, 71, 71)),
                                    )
                                    .center(iced::Length::Fill),
                                )
                                .padding(0)
                                .width(32.0)
                                .height(32.0)
                                .style(themed_button)
                                .on_press(HomeMessage::DeleteCollection(id)),
                            )
                            .push(
                                button(
                                    container(icon(0xe5cd).size(20.0)).center(iced::Length::Fill),
                                )
                                .padding(0)
                                .width(32.0)
                                .height(32.0)
                                .style(themed_button)
                                .on_press(HomeMessage::CancelSidebarAction),
                            ),
                    )
                    .into(),
                    _ => hover(
                        sidebar_button(0xe04a, text(truncate_text(collection.name(), 25)))
                            .on_press(HomeMessage::Goto(Tab::Collection(id))),
                        row![]
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .padding(iced::Padding::new(0.0).right(5.0))
                            .align_y(iced::Alignment::Center)
                            .push(horizontal_space())
                            .push_maybe((!matches!(action, Action::DeleteCollection(_))).then(
                                || {
                                    menu_button(
                                        container(icon(0xe5d2).size(20.0))
                                            .center(iced::Length::Fill),
                                        move || {
                                            container(
                                                column![]
                                                    .width(150.0)
                                                    .spacing(5.0)
                                                    .push(menu_item(0xe3c9, "Rename").on_press(
                                                        HomeMessage::BeginRenameCollection(id),
                                                    ))
                                                    .push(menu_item(0xe872, "Delete").on_press(
                                                        HomeMessage::BeginDeleteCollection(id),
                                                    )),
                                            )
                                            .padding(5.0)
                                            .style(|theme: &iced::Theme| container::Style {
                                                background: Some(iced::Background::Color(
                                                    theme.extended_palette().background.strong.text,
                                                )),
                                                border: iced::Border {
                                                    color: theme
                                                        .extended_palette()
                                                        .background
                                                        .weak
                                                        .color,
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
                                    .width(32.0)
                                    .height(32.0)
                                    .style(themed_button)
                                },
                            )),
                    )
                    .into(),
                }
            }))
            .push(
                sidebar_button(
                    0xe145,
                    text("New collection").color(iced::Color::WHITE.scale_alpha(0.5)),
                )
                .on_press(HomeMessage::NewCollection),
            )
            .push(vertical_space())
            .push(
                container(
                    text(format!("Jangal v{}", env!("CARGO_PKG_VERSION")))
                        .color(iced::Color::from_rgb8(100, 100, 100)),
                )
                .padding(iced::Padding::new(0.0).left(5.0)),
            )
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
                        .style(themed_button),
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

fn sidebar_button<'a>(
    icon: u32,
    label: impl Into<iced::Element<'a, HomeMessage>>,
) -> iced::widget::Button<'a, HomeMessage> {
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
    .style(themed_button)
}
