use iced::widget::image;

use super::{
    AppState, LibraryStatus, Tab,
    screen::{self, Screen, cards},
};
use crate::{library, settings::UserSettings};
use std::{collections::VecDeque, time::Instant};

pub struct App {
    now: Instant,
    screen: AppScreen,
    state: AppState,
}

impl App {
    pub fn new() -> (Self, iced::Task<Message>) {
        let storage_path = directories::ProjectDirs::from("com", "Jangal", "Jangal")
            .expect("system storage directories")
            .data_local_dir()
            .to_path_buf();
        std::fs::create_dir_all(&storage_path).expect("mkdir");

        let library = library::Library::load(&storage_path);
        let settings = UserSettings::load(&storage_path);

        let (card_cache, cache_task) = cards::Cache::build(&library);

        let (screen, screen_task) = screen::Home::new();

        (
            App {
                now: Instant::now(),
                screen: AppScreen::Home(screen),
                state: AppState {
                    storage_path,
                    library,
                    settings,

                    card_cache,
                    library_status: LibraryStatus::Idle,
                    tab_stack: VecDeque::from([Tab::Home]),
                },
            },
            iced::Task::batch([cache_task, screen_task.map(Message::Home)]),
        )
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            match &self.screen {
                AppScreen::Home(screen) => screen.subscription(self.now).map(Message::Home),
                AppScreen::Player(screen) => screen.subscription(self.now).map(Message::Player),
                AppScreen::Settings(screen) => screen.subscription(self.now).map(Message::Settings),
            },
            iced::event::listen_with(|event, _, _| match event {
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Exit),
                _ => None,
            }),
            self.state.card_cache.subscription(self.now),
        ])
    }

    pub fn update(&mut self, message: Message, now: Instant) -> iced::Task<Message> {
        self.now = now;

        match message {
            Message::Home(screen::HomeMessage::Play(id)) => {
                let (screen, task) = screen::Player::new(id, &self.state);
                self.screen = AppScreen::Player(screen);
                task.map(Message::Player)
            }
            Message::Home(screen::HomeMessage::OpenSettings) => {
                let (screen, task) = screen::Settings::new();
                self.screen = AppScreen::Settings(screen);
                task.map(Message::Settings)
            }
            Message::Home(screen::HomeMessage::Action(action)) => match action {
                screen::HomeAction::Purge => iced::Task::done(Message::Purge { scan: false }),
                screen::HomeAction::ScanDirectories => {
                    iced::Task::done(Message::Purge { scan: true })
                }
                screen::HomeAction::ForceScan => iced::Task::done(Message::Scrape { force: true }),
            },
            Message::Player(screen::PlayerMessage::Back) => {
                let (screen, task) = screen::Home::new();
                self.screen = AppScreen::Home(screen);
                iced::Task::batch([
                    iced::window::latest()
                        .and_then(|id| iced::window::set_mode(id, iced::window::Mode::Windowed)),
                    task.map(Message::Home),
                ])
            }
            Message::Settings(screen::SettingsMessage::Back) => {
                let (screen, task) = screen::Home::new();
                self.screen = AppScreen::Home(screen);
                iced::Task::batch([
                    iced::Task::perform(self.state.save_settings(), |_| ()).discard(),
                    task.map(Message::Home),
                ])
            }
            Message::Home(message) => {
                let AppScreen::Home(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen
                    .update(message, &mut self.state, self.now)
                    .map(Message::Home)
            }
            Message::Settings(message) => {
                let AppScreen::Settings(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen
                    .update(message, &mut self.state, self.now)
                    .map(Message::Settings)
            }
            Message::Player(message) => {
                let AppScreen::Player(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen
                    .update(message, &mut self.state, self.now)
                    .map(|message| Message::Player(message))
            }
            Message::Purge { scan } => {
                self.state.library_status = LibraryStatus::Scanning;

                let existing: Vec<_> = self
                    .state
                    .library
                    .iter()
                    .filter_map(|(id, media)| Some((*id, media.video()?.path.to_path_buf())))
                    .collect();

                iced::Task::perform(
                    async move { library::purge_media(existing.into_iter()).await },
                    move |removed| Message::PurgeComplete { scan, removed },
                )
            }
            Message::ScanDirectories => {
                self.state.library_status = LibraryStatus::Scanning;

                let directories = self.state.settings.directories.clone();

                iced::Task::perform(
                    async move {
                        library::scan_directories(directories.iter().map(|path| path.as_path()))
                            .await
                            .unwrap_or_default()
                    },
                    Message::ScanDirectoriesComplete,
                )
            }
            Message::Scrape { force } => {
                self.state.library_status = LibraryStatus::Scanning;

                let storage = self.state.storage_path.clone();
                let tmdb_secret = self.state.settings.tmdb_secret.clone();
                let media: Vec<_> = self
                    .state
                    .library
                    .iter_mut()
                    .filter_map(|(id, media)| match media {
                        library::Media::Uncategorised(uncategorised)
                            if force || !uncategorised.dont_scrape =>
                        {
                            uncategorised.dont_scrape = true;
                            Some((
                                *id,
                                uncategorised.video.path.file_name()?.to_str()?.to_string(),
                            ))
                        }
                        _ => None,
                    })
                    .collect();

                iced::Task::perform(
                    async move {
                        let scraper = library::TmdbScraper::new(&tmdb_secret);
                        library::scrape_all(&scraper, &storage, media.into_iter()).await
                    },
                    Message::ScrapeComplete,
                )
            }
            Message::PurgeComplete { scan, removed } => {
                for id in removed {
                    self.state.library.remove(id);
                }
                self.state.library.purge_collections();
                self.state.library.save(&self.state.storage_path).unwrap();
                let (card_cache, cache_task) = cards::Cache::build(&self.state.library);
                self.state.card_cache = card_cache;
                iced::Task::batch([
                    cache_task,
                    if scan {
                        iced::Task::done(Message::ScanDirectories)
                    } else {
                        self.state.library_status = LibraryStatus::Idle;
                        iced::Task::none()
                    },
                ])
            }
            Message::ScanDirectoriesComplete(added) => {
                self.state.library.extend(added);
                self.state.library.save(&self.state.storage_path).unwrap();
                iced::Task::done(Message::Scrape { force: false })
            }
            Message::ScrapeComplete(result) => {
                self.state.library_status = LibraryStatus::Idle;
                let (card_cache, cache_task) = cards::Cache::build(&self.state.library);
                self.state.card_cache = card_cache;
                result.insert(&mut self.state.library);
                iced::Task::batch([
                    cache_task,
                    iced::Task::perform(self.state.save_library(), |_| ()).discard(),
                ])
            }
            Message::CardImageLoaded(id, image) => {
                if let Some(image) = image {
                    self.state.card_cache.load_image(id, image);
                }
                iced::Task::none()
            }
            Message::Animate => iced::Task::none(),
            Message::Exit => iced::Task::batch([
                iced::Task::perform(self.state.save_library(), |_| ()),
                iced::Task::perform(self.state.save_settings(), |_| ()),
            ])
            .chain(iced::exit())
            .discard(),
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        match &self.screen {
            AppScreen::Home(screen) => screen.view(&self.state, self.now).map(Message::Home),
            AppScreen::Player(screen) => screen.view(&self.state, self.now).map(Message::Player),
            AppScreen::Settings(screen) => {
                screen.view(&self.state, self.now).map(Message::Settings)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Home(screen::HomeMessage),
    Player(screen::PlayerMessage),
    Settings(screen::SettingsMessage),

    Purge {
        scan: bool,
    },
    ScanDirectories,
    Scrape {
        force: bool,
    },
    PurgeComplete {
        scan: bool,
        removed: Vec<library::MediaId>,
    },
    ScanDirectoriesComplete(Vec<library::Media>),
    ScrapeComplete(library::ScrapeResult),
    CardImageLoaded(library::MediaId, Option<image::Allocation>),

    Animate,
    Exit,
}

pub enum AppScreen {
    Home(screen::Home),
    Player(screen::Player),
    Settings(screen::Settings),
}
