pub mod home;
mod player;
mod settings;

use std::time::Instant;

pub use home::*;
pub use player::*;
pub use settings::*;

use super::AppState;

pub trait Screen {
    type Message;

    fn update(
        &mut self,
        message: Self::Message,
        state: &mut AppState,
        now: Instant,
    ) -> iced::Task<Self::Message>;
    fn view<'a, 'b>(
        &'a self,
        state: &'a AppState,
        now: Instant,
    ) -> iced::Element<'b, Self::Message>
    where
        'a: 'b;

    fn subscription(&self, _now: Instant) -> iced::Subscription<Self::Message> {
        iced::Subscription::none()
    }
}
