use super::{
    screen::{self, Screen},
    AppState, LibraryStatus,
};
use crate::{
    library::{self, Scraper},
    settings::UserSettings,
};
use futures::future;

pub struct App {
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

        let (screen, task) = screen::Home::new();

        (
            App {
                screen: AppScreen::Home(screen),
                state: AppState {
                    storage_path,
                    library,
                    settings,

                    library_status: LibraryStatus::Idle,
                },
            },
            task.map(Message::Home),
        )
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Home(screen::HomeMessage::SelectMovie(id)) => {
                self.screen = AppScreen::Player(screen::Player::new(id, &self.state));
                iced::Task::none()
            }
            Message::Home(screen::HomeMessage::OpenSettings) => {
                let (screen, task) = screen::Settings::new();
                self.screen = AppScreen::Settings(screen);
                task.map(Message::Settings)
            }
            Message::Home(screen::HomeMessage::ScanDirectoriesComplete { removed, added }) => {
                for id in removed {
                    self.state.library.remove(id);
                }
                self.state.library.extend(added);
                self.state.library.save(&self.state.storage_path).unwrap();

                let storage = self.state.storage_path.clone();
                let tmdb_secret = self.state.settings.tmdb_secret.clone();
                let media: Vec<_> = self
                    .state
                    .library
                    .iter()
                    .filter_map(|(id, media)| match media {
                        library::Media::Uncategorised(path) => {
                            Some((*id, path.file_name()?.to_str()?.to_string()))
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
            Message::Player(screen::PlayerMessage::Back) => {
                let (screen, task) = screen::Home::new();
                self.screen = AppScreen::Home(screen);
                task.map(Message::Home)
            }
            Message::Settings(screen::SettingsMessage::Back) => {
                let (screen, task) = screen::Home::new();
                self.screen = AppScreen::Home(screen);
                task.map(Message::Home)
            }
            Message::Home(message) => {
                let AppScreen::Home(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen.update(message, &mut self.state).map(Message::Home)
            }
            Message::Settings(message) => {
                let AppScreen::Settings(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen
                    .update(message, &mut self.state)
                    .map(Message::Settings)
            }
            Message::Player(message) => {
                let AppScreen::Player(screen) = &mut self.screen else {
                    return iced::Task::none();
                };
                screen
                    .update(message)
                    .map(|message| Message::Player(message))
            }
            Message::ScrapeComplete(result) => {
                result.insert(&mut self.state.library);

                self.state.library.save(&self.state.storage_path).unwrap();
                self.state.library_status = LibraryStatus::Idle;

                iced::Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        match &self.screen {
            AppScreen::Home(screen) => screen.view(&self.state).map(Message::Home),
            AppScreen::Player(screen) => screen.view(&self.state).map(Message::Player),
            AppScreen::Settings(screen) => screen.view(&self.state).map(Message::Settings),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Home(screen::HomeMessage),
    Player(screen::PlayerMessage),
    Settings(screen::SettingsMessage),
    ScrapeComplete(library::ScrapeResult),
}

pub enum AppScreen {
    Home(screen::Home),
    Player(screen::Player),
    Settings(screen::Settings),
}
