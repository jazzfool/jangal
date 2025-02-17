use super::Screen;
use crate::ui::{
    icon, themed_button, themed_text_input, AppState, HEADER_FONT, MONO_FONT, SUBTITLE_FONT,
};
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, rule, scrollable, slider,
    text, text_input,
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
            SettingsMessage::WatchThresholdMovies(minutes) => {
                if let Ok(minutes) = minutes.parse() {
                    state.settings.watch_threshold_movies = minutes;
                }
                iced::Task::none()
            }
            SettingsMessage::WatchThresholdEpisodes(minutes) => {
                if let Ok(minutes) = minutes.parse() {
                    state.settings.watch_threshold_episodes = minutes;
                }
                iced::Task::none()
            }
            SettingsMessage::SubtitleOpacity(opacity) => {
                state.settings.subtitle_opacity = opacity;
                iced::Task::none()
            }
            SettingsMessage::SubtitleSize(size) => {
                state.settings.subtitle_size = size;
                iced::Task::none()
            }
            SettingsMessage::ThumbnailInterval(interval) => {
                state.settings.thumbnail_interval = interval;
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view<'a, 'b>(&'a self, state: &'a AppState) -> iced::Element<'b, SettingsMessage>
    where
        'a: 'b,
    {
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
                                    .style(themed_button)
                                    .on_press_maybe(
                                        (!self.dialog_open).then_some(SettingsMessage::Back),
                                    ),
                            )
                            .push(
                                text("Settings")
                                    .font(HEADER_FONT)
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
                                        .style(themed_text_input)
                                        .font(MONO_FONT)
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
                                                    .spacing(10.0)
                                                    .push(
                                                        button(icon(0xe15b).size(20.0))
                                                            .style(themed_button)
                                                            .on_press(
                                                                SettingsMessage::RemoveDirectory(i),
                                                            ),
                                                    )
                                                    .push(
                                                        text(path.to_str().unwrap().to_string())
                                                            .font(MONO_FONT),
                                                    )
                                                    .push(horizontal_space())
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
                                            .style(themed_button)
                                            .on_press_maybe(
                                                (!self.dialog_open)
                                                    .then_some(SettingsMessage::AddDirectory),
                                            ),
                                        ),
                                ),
                        )
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(
                                    text("Fully watched threshold (Movies)")
                                        .width(iced::Length::FillPortion(1)),
                                )
                                .push(
                                    row![]
                                        .align_y(iced::Alignment::Center)
                                        .width(iced::Length::FillPortion(2))
                                        .spacing(5.0)
                                        .push(
                                            text_input(
                                                "",
                                                &state.settings.watch_threshold_movies.to_string(),
                                            )
                                            .align_x(iced::Alignment::End)
                                            .width(100)
                                            .on_input(SettingsMessage::WatchThresholdMovies)
                                            .style(themed_text_input),
                                        )
                                        .push("minute(s)")
                                        .push(horizontal_space()),
                                ),
                        )
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(
                                    text("Fully watched threshold (TV Episodes)")
                                        .width(iced::Length::FillPortion(1)),
                                )
                                .push(
                                    row![]
                                        .align_y(iced::Alignment::Center)
                                        .width(iced::Length::FillPortion(2))
                                        .spacing(5.0)
                                        .push(
                                            text_input(
                                                "",
                                                &state
                                                    .settings
                                                    .watch_threshold_episodes
                                                    .to_string(),
                                            )
                                            .align_x(iced::Alignment::End)
                                            .width(100)
                                            .on_input(SettingsMessage::WatchThresholdEpisodes)
                                            .style(themed_text_input),
                                        )
                                        .push("minute(s)")
                                        .push(horizontal_space()),
                                ),
                        )
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(text("Subtitle Opacity").width(iced::Length::FillPortion(1)))
                                .push(
                                    row![]
                                        .width(iced::Length::FillPortion(2))
                                        .align_y(iced::Alignment::Center)
                                        .spacing(5.0)
                                        .push(
                                            slider(
                                                0.1..=1.0,
                                                state.settings.subtitle_opacity,
                                                SettingsMessage::SubtitleOpacity,
                                            )
                                            .width(100.0)
                                            .step(0.05),
                                        )
                                        .push(text(format!(
                                            "{}%",
                                            (state.settings.subtitle_opacity * 100.0) as u32,
                                        )))
                                        .push(horizontal_space()),
                                ),
                        )
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(text("Subtitle Size").width(iced::Length::FillPortion(1)))
                                .push(
                                    row![]
                                        .width(iced::Length::FillPortion(2))
                                        .align_y(iced::Alignment::Center)
                                        .spacing(5.0)
                                        .push(
                                            slider(
                                                12.0..=48.0,
                                                state.settings.subtitle_size,
                                                SettingsMessage::SubtitleSize,
                                            )
                                            .width(100.0)
                                            .step(1.0),
                                        )
                                        .push(text(state.settings.subtitle_size as u32))
                                        .push(horizontal_space()),
                                ),
                        )
                        .push(
                            container(
                                container(
                                    text("This is how subtitle text will appear")
                                        .font(SUBTITLE_FONT)
                                        .size(state.settings.subtitle_size)
                                        .color(iced::Color::from_rgb8(231, 211, 73)),
                                )
                                .padding(iced::Padding::new(5.0).left(10.0).right(10.0))
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Color(
                                        iced::Color::BLACK
                                            .scale_alpha(state.settings.subtitle_opacity),
                                    )),
                                    ..Default::default()
                                }),
                            )
                            .padding(iced::Padding::new(15.0).left(20.0).right(20.0))
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(iced::Color::from_rgb(
                                    0.5, 0.5, 0.5,
                                ))),
                                border: iced::Border::default().rounded(5.0),
                                ..Default::default()
                            }),
                        )
                        .push(
                            row![]
                                .align_y(iced::Alignment::Center)
                                .push(
                                    text("Thumbnail Interval").width(iced::Length::FillPortion(1)),
                                )
                                .push(
                                    row![]
                                        .width(iced::Length::FillPortion(2))
                                        .align_y(iced::Alignment::Center)
                                        .spacing(5.0)
                                        .push(
                                            slider(
                                                0..=1800,
                                                state.settings.thumbnail_interval,
                                                SettingsMessage::ThumbnailInterval,
                                            )
                                            .width(100.0)
                                            .step(1u32),
                                        )
                                        .push(text(format!(
                                            "{}s",
                                            state.settings.thumbnail_interval
                                        )))
                                        .push(horizontal_space()),
                                ),
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
    WatchThresholdMovies(String),
    WatchThresholdEpisodes(String),
    SubtitleOpacity(f32),
    SubtitleSize(f32),
    ThumbnailInterval(u32),
}
