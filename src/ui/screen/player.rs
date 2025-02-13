mod seekbar;

use crate::{
    library,
    ui::{clear_button, clear_scrollable, icon, menu_button, AppState},
};
use gstreamer::prelude::{ElementExt, ObjectExt};
use iced::widget::{
    button, center, column, container, horizontal_space, image, mouse_area, row, scrollable,
    slider, stack, text, vertical_space,
};
use iced_video_player::{Position, Video, VideoPlayer};
use rfd::AsyncFileDialog;
use std::{
    num::NonZeroU8,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

fn keep_awake() -> keepawake::KeepAwake {
    keepawake::Builder::default()
        .display(true)
        .reason("Video Playback")
        .app_name("Jangal")
        .app_reverse_domain("io.github.jangal")
        .create()
        .expect("keep awake")
}

fn load_video(path: &Path) -> Video {
    let mut video = Video::new(&url::Url::from_file_path(path).unwrap()).unwrap();
    video.set_paused(true);
    video
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SubtitleOption {
    None,
    Stream(usize),
    File(PathBuf),
}

pub struct Player {
    id: library::MediaId,
    video: Option<Video>,
    duration: f64,
    position: f64,
    dragging: bool,
    show_controls: bool,
    is_fullscreen: bool,
    subtitle_streams: Vec<String>,
    thumbnails: Vec<image::Handle>,
    subtitle: Option<String>,
    selected_subtitle: SubtitleOption,
    subtitle_menu_open: bool,
    dialog_open: bool,
    _keep_awake: Option<keepawake::KeepAwake>,
}

impl Player {
    pub fn new(id: library::MediaId, state: &AppState) -> (Self, iced::Task<PlayerMessage>) {
        let media = state.library.get(id).unwrap();
        let media = media.video().unwrap();

        let media_path = media.path.clone();
        let video_task = iced::Task::perform(
            tokio::task::spawn_blocking(move || {
                let video = load_video(&media_path);

                let pipeline = video.pipeline();
                let num_subtitles = pipeline.property::<i32>("n-text");
                let subtitle_streams = (0..num_subtitles)
                    .map(|i| {
                        let tags = pipeline
                            .emit_by_name::<Option<gstreamer::TagList>>("get-text-tags", &[&i]);

                        let name = tags.map(|tags| {
                            (
                                tags.get::<gstreamer::tags::LanguageCode>().and_then(|tag| {
                                    locale_codes::language::lookup(tag.get())
                                        .map(|lang| lang.reference_name.clone())
                                }),
                                tags.get::<gstreamer::tags::Title>()
                                    .map(|tag| tag.get().to_owned()),
                            )
                        });
                        match name {
                            Some((Some(language), Some(title))) => {
                                format!("{} | {}", language, title)
                            }
                            Some((Some(language), None)) => format!("{}", language),
                            Some((None, Some(title))) => format!("Stream {} | {}", i, title),
                            _ => format!("Stream {}", i),
                        }
                    })
                    .collect();

                (Arc::new(video), subtitle_streams)
            }),
            |res| {
                let res = res.unwrap();
                PlayerMessage::LoadVideo {
                    video: res.0,
                    subtitle_streams: res.1,
                }
            },
        );

        let thumbnail_task = if state.settings.thumbnail_interval > 0 {
            let media_path = media.path.clone();
            let thumbnail_interval = state.settings.thumbnail_interval;
            iced::Task::perform(
                tokio::task::spawn_blocking(move || {
                    let mut headless = load_video(&media_path);
                    let duration = headless.duration().as_secs_f64();
                    let num_thumbnails = duration as u32 / thumbnail_interval;
                    headless
                        .thumbnails(
                            (0..num_thumbnails).map(|i| {
                                Position::Time(Duration::from_secs_f64(
                                    duration * (i as f64 / num_thumbnails as f64),
                                ))
                            }),
                            NonZeroU8::new(8).unwrap(/* invariant */),
                        )
                        .expect("thumbnails")
                }),
                |res| PlayerMessage::Thumbnails(res.unwrap()),
            )
        } else {
            iced::Task::none()
        };

        (
            Player {
                id,
                video: None,
                duration: 0.0,
                position: 0.0,
                dragging: false,
                show_controls: false,
                is_fullscreen: false,
                subtitle_streams: vec![],
                thumbnails: vec![],
                subtitle: None,
                selected_subtitle: SubtitleOption::None,
                subtitle_menu_open: false,
                dialog_open: false,
                _keep_awake: Some(keep_awake()),
            },
            iced::Task::batch([video_task, thumbnail_task]),
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
            PlayerMessage::LoadVideo {
                video,
                subtitle_streams,
            } => {
                let mut video = Arc::try_unwrap(video).unwrap();

                let media = state.library.get(self.id).unwrap();
                let media = media.video().unwrap();

                let duration = video.duration().as_secs_f64();

                if let Some(position) = match media.watched {
                    library::Watched::Partial { seconds, .. } => {
                        Some(Position::Time(Duration::from_secs_f32(seconds)))
                    }
                    _ => None,
                } {
                    video.seek(position, true).unwrap();
                }

                video.set_paused(false);

                self.video = Some(video);

                self.duration = duration;
                self.subtitle_streams = subtitle_streams;

                iced::Task::none()
            }
            PlayerMessage::NewFrame => {
                let Some(video) = self.video.as_ref() else {
                    return iced::Task::none();
                };

                if !self.dragging {
                    self.position = video.position().as_secs_f64();
                }

                iced::Task::none()
            }
            PlayerMessage::Thumbnails(thumbnails) => {
                self.thumbnails = thumbnails;
                iced::Task::none()
            }
            PlayerMessage::NewSubtitle(subtitle) => {
                self.subtitle = subtitle;
                iced::Task::none()
            }
            PlayerMessage::Seek(secs) => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                self.dragging = true;
                video.set_paused(true);
                self.position = secs;
                video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .expect("seek");
                iced::Task::none()
            }
            PlayerMessage::SeekRelease => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                self.dragging = false;
                video.set_paused(false);
                iced::Task::none()
            }
            PlayerMessage::Volume(volume) => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                video.set_volume(volume);
                video.set_muted(false);
                iced::Task::none()
            }
            PlayerMessage::TogglePause => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                video.set_paused(!video.paused());
                self._keep_awake = (!video.paused()).then(|| keep_awake());
                iced::Task::none()
            }
            PlayerMessage::ToggleMute => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                video.set_muted(!video.muted());
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
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                self.position = (self.position - 10.0).max(0.0);
                video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .unwrap();
                iced::Task::none()
            }
            PlayerMessage::SkipForward => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                self.position = (self.position + 10.0).min(self.duration);
                video
                    .seek(Duration::from_secs_f64(self.position), true)
                    .unwrap();
                iced::Task::none()
            }
            PlayerMessage::VolumeUp => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                video.set_volume((video.volume() + 0.1).clamp(0.0, 1.0));
                iced::Task::none()
            }
            PlayerMessage::VolumeDown => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                video.set_volume((video.volume() - 0.1).clamp(0.0, 1.0));
                iced::Task::none()
            }
            PlayerMessage::ToggleSubtitleMenuOpen(open) => {
                self.subtitle_menu_open = open;
                iced::Task::none()
            }
            PlayerMessage::SelectSubtitleStream(stream) => {
                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };
                let paused = video.paused();
                let position = video.position();
                video.pipeline().set_state(gstreamer::State::Ready).unwrap();
                video
                    .pipeline()
                    .set_property("suburi", Option::<&str>::None);
                video.pipeline().set_property("current-text", stream as i32);
                video.set_paused(paused);
                let _ = video.pipeline().state(None);
                video.seek(position, true).unwrap();

                self.subtitle = None;
                self.selected_subtitle = SubtitleOption::Stream(stream);

                iced::Task::none()
            }
            PlayerMessage::OpenSubtitleFilePicker => {
                if self.dialog_open {
                    return iced::Task::none();
                }
                self.dialog_open = true;

                iced::Task::perform(
                    async move {
                        AsyncFileDialog::new()
                            .add_filter("Subtitle files", &["srt"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    PlayerMessage::PickSubtitleFile,
                )
            }
            PlayerMessage::PickSubtitleFile(file) => {
                self.dialog_open = false;

                let Some(file) = file else {
                    return iced::Task::none();
                };

                let Some(video) = self.video.as_mut() else {
                    return iced::Task::none();
                };

                let position = video.position();
                video
                    .set_subtitle_url(&url::Url::from_file_path(&file).unwrap())
                    .expect("failed to set subtitle file");
                let _ = video.pipeline().state(None);
                video.seek(position, true).unwrap();

                self.subtitle = None;
                self.selected_subtitle = SubtitleOption::File(file);

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
                    center(if let Some(video) = &self.video {
                        VideoPlayer::new(video)
                            .on_new_frame(PlayerMessage::NewFrame)
                            .on_subtitle_text(PlayerMessage::NewSubtitle)
                            .content_fit(iced::ContentFit::Contain)
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .into()
                    } else {
                        iced::Element::from(text("Loading..."))
                    })
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(iced::Color::BLACK)),
                        ..Default::default()
                    }),
                )
                .push_maybe(
                    state
                        .settings
                        .show_subtitles
                        .then_some(())
                        .and(self.subtitle.as_ref())
                        .map(|subtitle| {
                            container(
                                container(
                                    text(subtitle.clone()).size(state.settings.subtitle_size),
                                )
                                .padding(iced::Padding::new(10.0).left(15.0).right(15.0))
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Color(
                                        iced::Color::BLACK
                                            .scale_alpha(state.settings.subtitle_opacity),
                                    )),
                                    border: iced::Border::default().rounded(10.0),
                                    ..Default::default()
                                }),
                            )
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .align_x(iced::Alignment::Center)
                            .align_y(iced::Alignment::End)
                            .padding(iced::Padding::ZERO.bottom(100.0))
                        }),
                )
                .push(
                    column![]
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .push(top_bar(
                            self.show_controls || self.subtitle_menu_open || self.dialog_open,
                            title,
                            self.is_fullscreen,
                        ))
                        .push(vertical_space().height(iced::Length::Fill))
                        .push_maybe(self.video.as_ref().map(|video| {
                            bottom_bar(
                                self.show_controls || self.subtitle_menu_open || self.dialog_open,
                                self.position,
                                self.subtitle_streams.clone(),
                                self.selected_subtitle.clone(),
                                self.thumbnails.clone(),
                                video,
                                state,
                            )
                        })),
                ),
        )
        .on_press(PlayerMessage::TogglePause)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum PlayerMessage {
    LoadVideo {
        video: Arc<Video>,
        subtitle_streams: Vec<String>,
    },
    NewFrame,
    Thumbnails(Vec<image::Handle>),
    NewSubtitle(Option<String>),
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
    ToggleSubtitleMenuOpen(bool),
    SelectSubtitleStream(usize),
    OpenSubtitleFilePicker,
    PickSubtitleFile(Option<PathBuf>),
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
    subtitle_streams: Vec<String>,
    selected_subtitle: SubtitleOption,
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

    fn media_controls<'a>(
        video: &Video,
        state: &AppState,
        subtitle_streams: Vec<String>,
        selected_subtitle: SubtitleOption,
    ) -> iced::Element<'a, PlayerMessage> {
        let subtitle_file = if let SubtitleOption::File(path) = &selected_subtitle {
            path.file_name()
                .map(|filename| filename.to_string_lossy().to_string())
        } else {
            None
        };

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
                    ))
                    .push(
                        menu_button(
                            container(
                                icon(0xe5c7)
                                    .size(30.0)
                                    .color(iced::Color::from_rgb8(220, 220, 220)),
                            )
                            .center(iced::Length::Fill),
                            move || {
                                let subtitle_streams = subtitle_streams.clone();
                                let subtitle_file = subtitle_file.clone();
                                container(
                                    scrollable(
                                        column![]
                                            .push(
                                                button(
                                                    row![]
                                                        .spacing(5.0)
                                                        .push(icon(0xeaf3).size(16.0).color(
                                                            iced::Color::from_rgb8(220, 220, 220),
                                                        ))
                                                        .push(text("Load subtitles from file")),
                                                )
                                                .on_press(PlayerMessage::OpenSubtitleFilePicker)
                                                .width(iced::Length::Fill)
                                                .height(30.0)
                                                .padding(iced::Padding::new(5.0).left(10.0))
                                                .style(clear_button),
                                            )
                                            .push(
                                                subtitle_file
                                                    .map(|subtitle_file| {
                                                        iced::Element::from(
                                                            row![]
                                                                .spacing(5.0)
                                                                .width(iced::Length::Fill)
                                                                .height(30.0)
                                                                .padding(
                                                                    iced::Padding::new(5.0)
                                                                        .left(10.0),
                                                                )
                                                                .push(
                                                                    icon(0xe5ca).size(16.0).color(
                                                                        iced::Color::from_rgb8(
                                                                            220, 220, 220,
                                                                        ),
                                                                    ),
                                                                )
                                                                .push(text(subtitle_file).color(
                                                                    iced::Color::from_rgba8(
                                                                        220, 220, 220, 0.5,
                                                                    ),
                                                                )),
                                                        )
                                                    })
                                                    .unwrap_or_else(|| row![].into()),
                                            )
                                            .extend(subtitle_streams.into_iter().enumerate().map(
                                                |(i, name)| {
                                                    button(
                                                        row![]
                                                            .spacing(5.0)
                                                            .push(icon(0xe5ca).size(16.0).color(
                                                                iced::Color::from_rgba8(
                                                                    220,
                                                                    220,
                                                                    220,
                                                                    if selected_subtitle
                                                                        == SubtitleOption::Stream(i)
                                                                    {
                                                                        1.0
                                                                    } else {
                                                                        0.0
                                                                    },
                                                                ),
                                                            ))
                                                            .push(text(name)),
                                                    )
                                                    .on_press(PlayerMessage::SelectSubtitleStream(
                                                        i,
                                                    ))
                                                    .width(iced::Length::Fill)
                                                    .height(30.0)
                                                    .padding(iced::Padding::new(5.0).left(10.0))
                                                    .style(clear_button)
                                                    .into()
                                                },
                                            )),
                                    )
                                    .style(clear_scrollable),
                                )
                                .max_width(300.0)
                                .max_height(400.0)
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
                        .on_toggle(PlayerMessage::ToggleSubtitleMenuOpen)
                        .style(|_, status| button::Style {
                            background: match status {
                                button::Status::Hovered => Some(iced::Background::Color(
                                    iced::Color::from_rgba8(255, 255, 255, 0.01),
                                )),
                                _ => None,
                            },
                            border: iced::Border::default().rounded(5.0),
                            ..Default::default()
                        })
                        .padding(0.0)
                        .width(40.0)
                        .height(40.0)
                        .location(menu_button::Location::TopLeft),
                    ),
            )
            .into()
    }

    mouse_area(if show {
        container(
            column![]
                .spacing(15.0)
                .push(seek_controls(position, thumbnails, video))
                .push(media_controls(
                    video,
                    state,
                    subtitle_streams,
                    selected_subtitle,
                )),
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
