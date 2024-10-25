mod seekbar;

use crate::{
    library,
    ui::{icon, AppState},
};
use iced::widget::{
    button, center, column, container, horizontal_space, image, mouse_area, row, slider, stack,
    text, vertical_space,
};
use iced_video_player::{Position, Video, VideoPlayer};
use std::{num::NonZeroU8, path::Path, time::Duration};

fn keep_awake() -> keepawake::KeepAwake {
    keepawake::Builder::default()
        .display(true)
        .reason("Video Playback")
        .app_name("Jangal")
        .app_reverse_domain("io.github.jangal")
        .create()
        .expect("keep awake")
}

fn load_video(path: &Path, state: &AppState) -> Video {
    let mut video = Video::new(&url::Url::from_file_path(path).unwrap()).unwrap();
    video.set_subtitles_enabled(state.settings.show_subtitles);
    video.set_subtitle_font("Sans", 16);
    video
}

pub struct Player {
    id: library::MediaId,
    video: Video,
    duration: f64,
    position: f64,
    dragging: bool,
    show_controls: bool,
    is_fullscreen: bool,
    thumbnails: Vec<image::Handle>,
    _keep_awake: Option<keepawake::KeepAwake>,
}

impl Player {
    pub fn new(id: library::MediaId, state: &AppState) -> (Self, iced::Task<PlayerMessage>) {
        let media = state.library.get(id).unwrap();
        let media = media.video().unwrap();

        let mut video = load_video(&media.path, state);
        let duration = video.duration().as_secs_f64();

        if let Some(position) = match media.watched {
            library::Watched::Partial { seconds, .. } => {
                Some(Position::Time(Duration::from_secs_f32(seconds)))
            }
            _ => None,
        } {
            video.seek(position, true).unwrap();
        }

        let mut headless = load_video(&media.path, state);
        let thumbnails_fut = async move {
            headless
                .thumbnails(
                    (0..32).map(|i| {
                        Position::Time(Duration::from_secs_f64(duration * (i as f64 / 32.0)))
                    }),
                    NonZeroU8::new(8).unwrap(/* invariant */),
                )
                .expect("thumbnails")
        };

        (
            Player {
                id,
                video,
                duration,
                position: 0.0,
                dragging: false,
                show_controls: false,
                is_fullscreen: false,
                thumbnails: vec![],
                _keep_awake: Some(keep_awake()),
            },
            iced::Task::perform(thumbnails_fut, PlayerMessage::Thumbnails),
        )
    }

    pub fn subscription(&self) -> iced::Subscription<PlayerMessage> {
        iced::Subscription::batch([
            iced::time::every(Duration::from_secs(1)).map(|_| PlayerMessage::UpdateWatched),
            iced::time::every(Duration::from_secs(60)).map(|_| PlayerMessage::SaveLibrary),
            iced::keyboard::on_key_press(|key, _| match key {
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Space) => {
                    Some(PlayerMessage::TogglePause)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                    Some(PlayerMessage::SkipBackward)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                    Some(PlayerMessage::SkipForward)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    Some(PlayerMessage::VolumeUp)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    Some(PlayerMessage::VolumeDown)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::PageUp) => {
                    Some(PlayerMessage::Previous)
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::PageDown) => {
                    Some(PlayerMessage::Next)
                }
                _ => None,
            }),
        ])
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
            PlayerMessage::Thumbnails(thumbnails) => {
                self.thumbnails = thumbnails;
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
                self._keep_awake = (!self.video.paused()).then(|| keep_awake());
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
            PlayerMessage::ToggleSubtitles => {
                state.settings.show_subtitles = !state.settings.show_subtitles;
                self.video
                    .set_subtitles_enabled(state.settings.show_subtitles);
                state.settings.save(&state.storage_path).unwrap();
                iced::Task::none()
            }
            PlayerMessage::UpdateWatched => {
                if let Some((is_episode, video)) =
                    state.library.get_mut(self.id).and_then(|media| {
                        Some((
                            matches!(media, library::Media::Episode(_)),
                            media.video_mut()?,
                        ))
                    })
                {
                    let watched_threshold = if is_episode {
                        state.settings.watch_threshold_episodes
                    } else {
                        state.settings.watch_threshold_movies
                    };
                    let watched_threshold = watched_threshold as f64 * 60.0;

                    video.watched = if self.duration - self.position < watched_threshold {
                        library::Watched::Yes
                    } else {
                        library::Watched::Partial {
                            seconds: self.position as f32,
                            percent: (self.position / self.duration) as f32,
                        }
                    };
                    video.last_watched = Some(chrono::Local::now());
                }
                iced::Task::none()
            }
            PlayerMessage::SaveLibrary => {
                iced::Task::perform(state.save_library(), |_| ()).discard()
            }
            PlayerMessage::Previous => {
                if let Some(previous) = library::previous_in_list(self.id, &state.library) {
                    let Player { is_fullscreen, .. } = *self;
                    let (screen, task) = Player::new(previous, state);
                    *self = screen;
                    self.show_controls = true;
                    self.is_fullscreen = is_fullscreen;
                    task
                } else {
                    iced::Task::none()
                }
            }
            PlayerMessage::Next => {
                if let Some(next) = library::next_in_list(self.id, &state.library) {
                    let Player { is_fullscreen, .. } = *self;
                    let (screen, task) = Player::new(next, state);
                    *self = screen;
                    self.show_controls = true;
                    self.is_fullscreen = is_fullscreen;
                    task
                } else {
                    iced::Task::none()
                }
            }
            PlayerMessage::SkipBackward => {
                self.position = (self.position - 10.0).max(0.0);
                self.video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .unwrap();
                iced::Task::none()
            }
            PlayerMessage::SkipForward => {
                self.position = (self.position + 10.0).min(self.duration);
                self.video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .unwrap();
                iced::Task::none()
            }
            PlayerMessage::VolumeUp => {
                self.video
                    .set_volume((self.video.volume() + 0.1).clamp(0.0, 1.0));
                iced::Task::none()
            }
            PlayerMessage::VolumeDown => {
                self.video
                    .set_volume((self.video.volume() - 0.1).clamp(0.0, 1.0));
                iced::Task::none()
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
            Some(_) => library::full_title(self.id, &state.library),
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
                        .push(top_bar(self.show_controls, title, self.is_fullscreen))
                        .push(vertical_space().height(iced::Length::Fill))
                        .push(bottom_bar(
                            self.show_controls,
                            self.position,
                            self.thumbnails.clone(),
                            &self.video,
                            state,
                        )),
                ),
        )
        .on_press(PlayerMessage::TogglePause)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum PlayerMessage {
    NewFrame,
    Thumbnails(Vec<image::Handle>),
    Seek(f64),
    SeekRelease,
    Volume(f64),
    TogglePause,
    ToggleMute,
    Back,
    MouseEnter,
    MouseExit,
    ToggleFullscreen,
    ToggleSubtitles,
    UpdateWatched,
    SaveLibrary,
    Previous,
    Next,
    SkipBackward,
    SkipForward,
    VolumeUp,
    VolumeDown,
}

fn top_bar<'a>(show: bool, title: String, is_fullscreen: bool) -> iced::Element<'a, PlayerMessage> {
    mouse_area(if show {
        container(
            row![]
                .spacing(20.0)
                .align_y(iced::Alignment::Center)
                .push(control_button(icon(0xe5c4), PlayerMessage::Back, false))
                .push(text(title))
                .push(horizontal_space())
                .push(control_button(
                    icon(if is_fullscreen { 0xe5d1 } else { 0xe5d0 }),
                    PlayerMessage::ToggleFullscreen,
                    false,
                )),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(0.0)
                    .add_stop(0.0, iced::Color::from_rgba8(0, 0, 0, 0.0))
                    .add_stop(1.0, iced::Color::from_rgba8(0, 0, 0, 0.8)),
            ))),
            ..Default::default()
        })
        .padding(iced::Padding::ZERO.left(20.0).right(20.0))
        .align_y(iced::Alignment::Center)
        .width(iced::Length::Fill)
        .height(60.0)
        .into()
    } else {
        iced::Element::from(vertical_space().width(iced::Length::Fill).height(60.0))
    })
    .on_enter(PlayerMessage::MouseEnter)
    .on_exit(PlayerMessage::MouseExit)
    .into()
}

fn bottom_bar<'a>(
    show: bool,
    position: f64,
    thumbnails: Vec<image::Handle>,
    video: &Video,
    state: &AppState,
) -> iced::Element<'a, PlayerMessage> {
    fn seek_controls<'a>(
        position: f64,
        thumbnails: Vec<image::Handle>,
        video: &Video,
    ) -> iced::Element<'a, PlayerMessage> {
        row![]
            .align_y(iced::Alignment::Center)
            .spacing(10.0)
            .push(
                text(format!(
                    "{:02}:{:02}:{:02}",
                    position as u64 / 3600,
                    position as u64 % 3600 / 60,
                    position as u64 % 60
                ))
                .width(80.0),
            )
            .push(
                seekbar::seekbar(
                    0.0..=video.duration().as_secs_f64(),
                    video.duration(),
                    position,
                    thumbnails,
                    PlayerMessage::Seek,
                )
                .step(0.1)
                .on_release(PlayerMessage::SeekRelease),
            )
            .push(
                text(format!(
                    "{:02}:{:02}:{:02}",
                    video.duration().as_secs() as u64 / 3600,
                    video.duration().as_secs() as u64 % 3600 / 60,
                    video.duration().as_secs() as u64 % 60
                ))
                .width(80.0)
                .align_x(iced::Alignment::End),
            )
            .into()
    }

    fn media_controls<'a>(video: &Video, state: &AppState) -> iced::Element<'a, PlayerMessage> {
        row![]
            .spacing(10.0)
            .align_y(iced::Alignment::Center)
            .push(
                // volume controls
                row![]
                    .align_y(iced::Alignment::Center)
                    .spacing(10.0)
                    .width(iced::Length::Fill)
                    .push(control_button(
                        icon(if video.muted() { 0xe04f } else { 0xe050 }),
                        PlayerMessage::ToggleMute,
                        true,
                    ))
                    .push(
                        slider(0.0..=1.0, video.volume(), PlayerMessage::Volume)
                            .step(0.05)
                            .width(100.0),
                    ),
            )
            // previous
            .push(control_button(icon(0xe045), PlayerMessage::Previous, true))
            // skip back
            .push(control_button(
                icon(0xe020),
                PlayerMessage::SkipBackward,
                false,
            ))
            // play/pause
            .push(control_button(
                icon(if video.paused() { 0xe037 } else { 0xe034 }),
                PlayerMessage::TogglePause,
                false,
            ))
            // skip forward
            .push(control_button(
                icon(0xe01f),
                PlayerMessage::SkipForward,
                false,
            ))
            // next
            .push(control_button(icon(0xe044), PlayerMessage::Next, true))
            // subtitle controls
            .push(
                row![]
                    .align_y(iced::Alignment::Center)
                    .spacing(10.0)
                    .width(iced::Length::Fill)
                    .push(horizontal_space())
                    .push(control_button(
                        icon(if state.settings.show_subtitles {
                            0xe048
                        } else {
                            0xef72
                        }),
                        PlayerMessage::ToggleSubtitles,
                        true,
                    )),
            )
            .into()
    }

    mouse_area(if show {
        container(
            column![]
                .spacing(15.0)
                .push(seek_controls(position, thumbnails, video))
                .push(media_controls(video, state)),
        )
        .padding(iced::Padding::new(20.0))
        .style(|_| container::Style {
            background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(0.0)
                    .add_stop(0.0, iced::Color::from_rgba8(0, 0, 0, 0.95))
                    .add_stop(1.0, iced::Color::from_rgba8(0, 0, 0, 0.0)),
            ))),
            ..Default::default()
        })
        .align_y(iced::Alignment::End)
        .width(iced::Length::Fill)
        .height(160.0)
        .into()
    } else {
        iced::Element::from(vertical_space().width(iced::Length::Fill).height(160.0))
    })
    .on_enter(PlayerMessage::MouseEnter)
    .on_exit(PlayerMessage::MouseExit)
    .into()
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
