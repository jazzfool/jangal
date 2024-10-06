mod home;
mod player;
mod settings;

pub use home::*;
pub use player::*;
pub use settings::*;

use super::AppState;

pub trait Screen {
    type Message;

    fn update(&mut self, message: Self::Message, state: &mut AppState)
        -> iced::Task<Self::Message>;
    fn view(&self, state: &AppState) -> iced::Element<Self::Message>;
}
