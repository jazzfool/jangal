use crate::{
    library,
    ui::{icon, AppState},
};
use iced::widget::{
    button, center, column, container, horizontal_space, mouse_area, row, slider, stack, svg, text,
    vertical_space,
};
use iced_video_player::{Position, Video, VideoPlayer};
use std::time::Duration;

pub struct Player {
    id: library::MediaId,
    video: Video,
    duration: f64,
    position: f64,
    dragging: bool,
    show_controls: bool,
    is_fullscreen: bool,
}

impl Player {
    pub fn new(id: library::MediaId, state: &AppState) -> (Self, iced::Task<PlayerMessage>) {
        let media = state.library.get(id).unwrap();
        let path = media.path().unwrap();

        let mut video = Video::new(&url::Url::from_file_path(path).unwrap()).unwrap();
        let duration = video.duration().as_secs_f64();

        if let Some(position) = media.watched().and_then(|watched| match watched {
            library::Watched::Partial { seconds, .. } => {
                Some(Position::Time(Duration::from_secs_f32(seconds)))
            }
            _ => None,
        }) {
            video.seek(position, true).unwrap();
        }

        (
            Player {
                id,
                video,
                duration,
                position: 0.0,
                dragging: false,
                show_controls: false,
                is_fullscreen: false,
            },
            iced::Task::none(),
        )
    }

    pub fn subscription(&self) -> iced::Subscription<PlayerMessage> {
        iced::time::every(Duration::from_secs(10)).map(|_| PlayerMessage::UpdateWatched)
    }

    pub fn update(
        &mut self,
        message: PlayerMessage,
        state: &mut AppState,
    ) -> iced::Task<PlayerMessage> {
        match message {
            PlayerMessage::NewFrame => {
                if !self.dragging {
                    self.position = self.video.position().as_secs_f64();
                }
                iced::Task::none()
            }
            PlayerMessage::Seek(secs) => {
                self.dragging = true;
                self.video.set_paused(true);
                self.position = secs;
                self.video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .expect("seek");
                iced::Task::none()
            }
            PlayerMessage::SeekRelease => {
                self.dragging = false;
                self.video.set_paused(false);
                iced::Task::none()
            }
            PlayerMessage::Volume(volume) => {
                self.video.set_volume(volume);
                self.video.set_muted(false);
                iced::Task::none()
            }
            PlayerMessage::TogglePause => {
                self.video.set_paused(!self.video.paused());
                iced::Task::none()
            }
            PlayerMessage::ToggleMute => {
                self.video.set_muted(!self.video.muted());
                iced::Task::none()
            }
            PlayerMessage::MouseEnter => {
                self.show_controls = true;
                iced::Task::none()
            }
            PlayerMessage::MouseExit => {
                self.show_controls = false;
                iced::Task::none()
            }
            PlayerMessage::ToggleFullscreen => {
                self.is_fullscreen = !self.is_fullscreen;
                let fullscreen = self.is_fullscreen;
                iced::window::get_latest()
                    .and_then(move |id| {
                        iced::window::change_mode::<()>(
                            id,
                            if fullscreen {
                                iced::window::Mode::Fullscreen
                            } else {
                                iced::window::Mode::Windowed
                            },
                        )
                    })
                    .discard()
            }
            PlayerMessage::UpdateWatched => {
                if let Some(watched) = state
                    .library
                    .get_mut(self.id)
                    .and_then(library::Media::watched_mut)
                {
                    // TODO: make fully-watched threshold adjustable
                    *watched = if self.duration - self.position < 120.0 {
                        library::Watched::Yes
                    } else {
                        library::Watched::Partial {
                            seconds: self.position as f32,
                            percent: (self.position / self.duration) as f32,
                        }
                    };
                    let library = state.library.clone();
                    let storage_path = state.storage_path.clone();
                    iced::Task::perform(
                        async move {
                            library.save(&storage_path).unwrap();
                        },
                        |_| (),
                    )
                    .discard()
                } else {
                    iced::Task::none()
                }
            }
            _ => iced::Task::none(),
        }
    }

    pub fn view<'a>(&'a self, state: &'a AppState) -> iced::Element<PlayerMessage> {
        let title = match state.library.get(self.id) {
            Some(library::Media::Episode(episode)) => {
                let series = state
                    .library
                    .get(episode.series)
                    .and_then(|media| match media {
                        library::Media::Series(series) => Some(series),
                        _ => None,
                    })
                    .map(|series| series.metadata.title.clone())
                    .unwrap_or("Unknown Series".into());
                format!(
                    "{} S{:02}E{:02} - {}",
                    series,
                    episode.metadata.season,
                    episode.metadata.episode,
                    episode.metadata.title
                )
            }
            Some(media) => media.full_title().unwrap_or("Unknown Media".into()),
            None => "Unknown Media".into(),
        };

        mouse_area(
            stack![]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .push(
                    center(
                        VideoPlayer::new(&self.video)
                            .on_new_frame(PlayerMessage::NewFrame)
                            .content_fit(iced::ContentFit::Contain)
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill),
                    )
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(iced::Color::BLACK)),
                        ..Default::default()
                    }),
                )
                .push(
                    column![]
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .push(
                            mouse_area(if self.show_controls {
                                container(
                                    row![]
                                        .spacing(20.0)
                                        .align_y(iced::Alignment::Center)
                                        .push(control_button(
                                            icon(0xe5c4),
                                            PlayerMessage::Back,
                                            false,
                                        ))
                                        .push(text(title))
                                        .push(horizontal_space())
                                        .push(control_button(
                                            icon(if self.is_fullscreen { 0xf1cf } else { 0xf1ce }),
                                            PlayerMessage::ToggleFullscreen,
                                            false,
                                        )),
                                )
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Gradient(
                                        iced::Gradient::Linear(
                                            iced::gradient::Linear::new(0.0)
                                                .add_stop(
                                                    0.0,
                                                    iced::Color::from_rgba8(0, 0, 0, 0.0),
                                                )
                                                .add_stop(
                                                    1.0,
                                                    iced::Color::from_rgba8(0, 0, 0, 0.8),
                                                ),
                                        ),
                                    )),
                                    ..Default::default()
                                })
                                .padding(iced::Padding::ZERO.left(20.0).right(20.0))
                                .align_y(iced::Alignment::Center)
                                .width(iced::Length::Fill)
                                .height(60.0)
                                .into()
                            } else {
                                iced::Element::from(
                                    vertical_space().width(iced::Length::Fill).height(60.0),
                                )
                            })
                            .on_enter(PlayerMessage::MouseEnter)
                            .on_exit(PlayerMessage::MouseExit),
                        )
                        .push(vertical_space().height(iced::Length::Fill))
                        .push(
                            mouse_area(if self.show_controls {
                                container(
                                    column![]
                                        .spacing(15.0)
                                        .push(
                                            row![]
                                                .align_y(iced::Alignment::Center)
                                                .spacing(10.0)
                                                .push(
                                                    text(format!(
                                                        "{:02}:{:02}:{:02}",
                                                        self.position as u64 / 3600,
                                                        self.position as u64 % 3600 / 60,
                                                        self.position as u64 % 60
                                                    ))
                                                    .width(80.0),
                                                )
                                                .push(
                                                    slider(
                                                        0.0..=self.video.duration().as_secs_f64(),
                                                        self.position,
                                                        PlayerMessage::Seek,
                                                    )
                                                    .step(0.1)
                                                    .style(|_theme: &iced::Theme, _| {
                                                        slider::Style {
                                                            rail: slider::Rail {
                                                                backgrounds: (
                                                                    iced::Background::Color(
                                                                        iced::Color::from_rgba8(
                                                                            245, 245, 245, 0.9,
                                                                        ),
                                                                    ),
                                                                    iced::Background::Color(
                                                                        iced::Color::from_rgba8(
                                                                            150, 150, 150, 0.7,
                                                                        ),
                                                                    ),
                                                                ),
                                                                width: 3.0,
                                                                border: iced::Border::default(),
                                                            },
                                                            handle: slider::Handle {
                                                                shape:
                                                                    slider::HandleShape::Rectangle {
                                                                        width: 7,
                                                                        border_radius:
                                                                            iced::border::radius(
                                                                                2.0,
                                                                            ),
                                                                    },
                                                                background: iced::Background::Color(
                                                                    iced::Color::from_rgba8(
                                                                        245, 245, 245, 0.9,
                                                                    ),
                                                                ),
                                                                border_width: 1.0,
                                                                border_color: iced::Color::BLACK,
                                                            },
                                                        }
                                                    })
                                                    .height(20.0)
                                                    .on_release(PlayerMessage::SeekRelease),
                                                )
                                                .push(
                                                    text(format!(
                                                        "{:02}:{:02}:{:02}",
                                                        self.video.duration().as_secs() as u64
                                                            / 3600,
                                                        self.video.duration().as_secs() as u64
                                                            % 3600
                                                            / 60,
                                                        self.video.duration().as_secs() as u64 % 60
                                                    ))
                                                    .width(80.0)
                                                    .align_x(iced::Alignment::End),
                                                ),
                                        )
                                        .push(
                                            row![]
                                                .spacing(10.0)
                                                .align_y(iced::Alignment::Center)
                                                .push(
                                                    row![]
                                                        .align_y(iced::Alignment::Center)
                                                        .spacing(10.0)
                                                        .width(iced::Length::Fill)
                                                        .push(control_button(
                                                            icon(if self.video.muted() {
                                                                0xe04f
                                                            } else {
                                                                0xe050
                                                            }),
                                                            PlayerMessage::ToggleMute,
                                                            true,
                                                        ))
                                                        .push(
                                                            slider(
                                                                0.0..=1.0,
                                                                self.video.volume(),
                                                                PlayerMessage::Volume,
                                                            )
                                                            .step(0.05)
                                                            .width(100.0),
                                                        ),
                                                )
                                                .push(control_button(
                                                    icon(0xe045),
                                                    PlayerMessage::NewFrame,
                                                    true,
                                                ))
                                                .push(control_button(
                                                    icon(0xe020),
                                                    PlayerMessage::NewFrame,
                                                    false,
                                                ))
                                                .push(control_button(
                                                    icon(if self.video.paused() {
                                                        0xe037
                                                    } else {
                                                        0xe034
                                                    }),
                                                    PlayerMessage::TogglePause,
                                                    false,
                                                ))
                                                .push(control_button(
                                                    icon(0xe01f),
                                                    PlayerMessage::NewFrame,
                                                    false,
                                                ))
                                                .push(control_button(
                                                    icon(0xe044),
                                                    PlayerMessage::NewFrame,
                                                    true,
                                                ))
                                                .push(horizontal_space()),
                                        ),
                                )
                                .padding(iced::Padding::ZERO.left(20.0).right(20.0))
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Gradient(
                                        iced::Gradient::Linear(
                                            iced::gradient::Linear::new(0.0)
                                                .add_stop(
                                                    0.0,
                                                    iced::Color::from_rgba8(0, 0, 0, 0.9),
                                                )
                                                .add_stop(
                                                    0.5,
                                                    iced::Color::from_rgba8(0, 0, 0, 0.7),
                                                )
                                                .add_stop(
                                                    1.0,
                                                    iced::Color::from_rgba8(0, 0, 0, 0.0),
                                                ),
                                        ),
                                    )),
                                    ..Default::default()
                                })
                                .align_y(iced::Alignment::Center)
                                .width(iced::Length::Fill)
                                .height(120.0)
                                .into()
                            } else {
                                iced::Element::from(
                                    vertical_space().width(iced::Length::Fill).height(120.0),
                                )
                            })
                            .on_enter(PlayerMessage::MouseEnter)
                            .on_exit(PlayerMessage::MouseExit),
                        ),
                ),
        )
        .on_press(PlayerMessage::TogglePause)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum PlayerMessage {
    NewFrame,
    Seek(f64),
    SeekRelease,
    Volume(f64),
    TogglePause,
    ToggleMute,
    Back,
    MouseEnter,
    MouseExit,
    ToggleFullscreen,
    UpdateWatched,
}

fn control_button(
    icon: iced::widget::Text,
    on_press: PlayerMessage,
    small: bool,
) -> iced::Element<PlayerMessage> {
    button(
        icon.size(if small { 26.0 } else { 30.0 })
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .color(iced::Color::from_rgb8(220, 220, 220)),
    )
    .style(|_, status| button::Style {
        background: match status {
            button::Status::Hovered => Some(iced::Background::Color(iced::Color::from_rgba8(
                255, 255, 255, 0.01,
            ))),
            _ => None,
        },
        border: iced::Border::default().rounded(5.0),
        ..Default::default()
    })
    .on_press(on_press)
    .padding(0.0)
    .width(40.0)
    .height(40.0)
    .into()
}
