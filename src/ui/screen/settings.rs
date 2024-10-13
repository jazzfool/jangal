use super::Screen;
use crate::ui::{clear_button, flat_text_input, icon, AppState, MONO_FONT};
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, rule, scrollable, text,
    text_input,
};
use normpath::PathExt;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

pub struct Settings {
    dialog_open: bool,
}

impl Settings {
    pub fn new() -> (Self, iced::Task<SettingsMessage>) {
        (Settings { dialog_open: false }, iced::Task::none())
    }
}

impl Screen for Settings {
    type Message = SettingsMessage;

    fn update(
        &mut self,
        message: SettingsMessage,
        state: &mut AppState,
    ) -> iced::Task<SettingsMessage> {
        match message {
            SettingsMessage::AddDirectory => {
                if self.dialog_open {
                    return iced::Task::none();
                }

                self.dialog_open = true;
                iced::Task::perform(
                    async move {
                        AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    SettingsMessage::AddDirectoryResponse,
                )
            }
            SettingsMessage::AddDirectoryResponse(path) => {
                self.dialog_open = false;

                if let Some(path) = path {
                    if !state
                        .settings
                        .directories
                        .iter()
                        .any(|other| other.normalize().unwrap() == path.normalize().unwrap())
                    {
                        state.settings.directories.push(path);
                    }
                }

                iced::Task::none()
            }
            SettingsMessage::RemoveDirectory(index) => {
                state.settings.directories.remove(index);
                iced::Task::none()
            }
            SettingsMessage::ApiSecretInput(secret) => {
                state.settings.tmdb_secret = secret;
                iced::Task::none()
            }
            SettingsMessage::Save => {
                state.settings.save(&state.storage_path).unwrap();
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view(&self, state: &AppState) -> iced::Element<SettingsMessage> {
        container(
            column![]
                .push(
                    container(
                        row![]
                            .width(iced::Length::Fill)
                            .align_y(iced::Alignment::Center)
                            .spacing(10.0)
                            .push(
                                button(icon(0xe5c4).size(26.0))
                                    .style(clear_button)
                                    .on_press_maybe(
                                        (!self.dialog_open).then_some(SettingsMessage::Back),
                                    ),
                            )
                            .push(
                                text("Settings")
                                    .size(26.0)
                                    .color(iced::Color::from_rgba8(210, 210, 210, 1.0)),
                            ),
                    )
                    .padding(10.0)
                    .style(|theme| container::Style {
                        background: Some(iced::Background::Color(theme.palette().background)),
                        ..Default::default()
                    }),
                )
                .push(horizontal_rule(1.0).style(|theme| rule::Style {
                    color: iced::Color::from_rgb8(40, 40, 40),
                    ..<iced::Theme as rule::Catalog>::default()(theme)
                }))
                .push(scrollable(
                    column![]
                        .spacing(10.0)
                        .padding(iced::Padding::new(20.0).left(25.0).right(25.0))
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(text("TMDB API Secret").width(iced::Length::FillPortion(1)))
                                .push(
                                    text_input("API Secret", &state.settings.tmdb_secret)
                                        .on_input(SettingsMessage::ApiSecretInput)
                                        .style(flat_text_input)
                                        .width(iced::Length::FillPortion(2)),
                                ),
                        )
                        .push(
                            row![]
                                .push(text("Media Directories").width(iced::Length::FillPortion(1)))
                                .push(
                                    column![]
                                        .width(iced::Length::FillPortion(2))
                                        .spacing(5.0)
                                        .extend(
                                            state
                                                .settings
                                                .directories
                                                .iter()
                                                .cloned()
                                                .enumerate()
                                                .map(|(i, path)| {
                                                    row![]
                                                    .align_y(iced::Alignment::Center)
                                                    .push(
                                                        text(path.to_str().unwrap().to_string())
                                                            .font(MONO_FONT),
                                                    )
                                                    .push(horizontal_space())
                                                    .push(
                                                        button(icon(0xe15b).size(20.0))
                                                            .style(clear_button)
                                                            .on_press(
                                                                SettingsMessage::RemoveDirectory(i),
                                                            ),
                                                    )
                                                    .into()
                                                }),
                                        )
                                        .push(
                                            button(
                                                row![]
                                                    .align_y(iced::Alignment::Center)
                                                    .spacing(10.0)
                                                    .push(icon(0xe145).size(20.0))
                                                    .push("Add"),
                                            )
                                            .width(iced::Length::Fill)
                                            .padding(iced::Padding::new(5.0).left(10.0))
                                            .style(clear_button)
                                            .on_press_maybe(
                                                (!self.dialog_open)
                                                    .then_some(SettingsMessage::AddDirectory),
                                            ),
                                        ),
                                ),
                        )
                        .push(
                            button(
                                row![]
                                    .align_y(iced::Alignment::Center)
                                    .spacing(10.0)
                                    .push(icon(0xe161).size(20.0))
                                    .push("Save"),
                            )
                            .padding(iced::Padding::new(5.0).left(10.0))
                            .style(clear_button)
                            .on_press_maybe((!self.dialog_open).then_some(SettingsMessage::Save)),
                        ),
                )),
        )
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Back,
    AddDirectory,
    AddDirectoryResponse(Option<PathBuf>),
    RemoveDirectory(usize),
    ApiSecretInput(String),
    Save,
}