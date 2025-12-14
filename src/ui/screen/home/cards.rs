use std::{
    f32,
    time::{Duration, Instant},
};

use super::{
    HomeMessage, Tab, collection_menu, media_menu, poster_image, search_maybe, watched_icon,
};
use crate::{
    library,
    ui::{SANS_FONT, app::Message, icon},
};
use iced::{
    Animation,
    advanced::text::Paragraph,
    animation::Easing,
    color,
    widget::{column, container, image, mouse_area, pin, row, space, stack, text},
};
use rustc_hash::FxHashMap;

pub struct Card {
    pub image: Option<image::Allocation>,
    pub hovered: bool,
    pub text_overflow: f32,
    pub text_animation: Option<Animation<f32>>,
    pub hover_animation: Animation<bool>,
}

impl Card {
    const CARD_WIDTH: f32 = 150.0;

    fn new(library: &library::Library, id: library::MediaId) -> Self {
        let text_width = iced_graphics::text::Paragraph::with_text(iced::advanced::Text {
            content: &library::full_title(id, library),
            bounds: iced::Size::new(f32::INFINITY, f32::INFINITY),
            size: 14.into(),
            line_height: text::LineHeight::Relative(1.0),
            font: SANS_FONT,
            align_x: text::Alignment::Left,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Auto,
            wrapping: text::Wrapping::None,
        })
        .min_width();

        let text_overflow = Self::CARD_WIDTH - text_width;

        let hover_animation = Animation::new(false)
            .very_quick()
            .easing(Easing::EaseInOutCubic);

        Card {
            image: None,
            hovered: false,
            text_overflow,
            text_animation: None,
            hover_animation,
        }
    }

    pub fn begin_hover(&mut self, now: Instant) {
        self.hovered = true;
        self.hover_animation.go_mut(true, now);
        if self.text_overflow < 0.0 {
            self.text_animation = Some(
                Animation::new(30.0)
                    .auto_reverse()
                    .repeat_forever()
                    .duration(Duration::from_secs_f32(-self.text_overflow / 20.))
                    .easing(Easing::Linear)
                    .go(self.text_overflow - 30.0, Instant::now()),
            );
        }
    }

    pub fn end_hover(&mut self, now: Instant) {
        self.hovered = false;
        self.hover_animation.go_mut(false, now);
        self.text_animation = None;
    }
}

pub struct Cache {
    pub cache: FxHashMap<library::MediaId, Card>,
}

impl Cache {
    pub fn build(library: &library::Library) -> (Self, iced::Task<Message>) {
        let mut cache = FxHashMap::default();

        let load_task = iced::Task::batch(library.iter().filter_map(|(id, media)| {
            cache.insert(*id, Card::new(library, *id));

            let poster = match media {
                library::Media::Episode(episode) => library.get(episode.season).unwrap().poster(),
                _ => media.poster(),
            };

            poster.map(|path| {
                let id = *id;
                image::allocate(path).map(move |result| Message::CardImageLoaded(id, result.ok()))
            })
        }));

        (Cache { cache }, load_task)
    }

    pub fn load_image(&mut self, id: library::MediaId, image: image::Allocation) {
        if let Some(card) = self.cache.get_mut(&id) {
            card.image = Some(image);
        }
    }

    pub fn subscription(&self, now: Instant) -> iced::Subscription<Message> {
        let is_animating = self.cache.iter().any(|(_id, card)| {
            card.text_animation
                .as_ref()
                .is_some_and(|anim| anim.is_animating(now) && card.hovered)
                || card.hover_animation.is_animating(now)
        });

        if is_animating {
            iced::window::frames().map(|_| Message::Animate)
        } else {
            iced::Subscription::none()
        }
    }
}

pub fn card_grid<'a, 'b>(
    cache: &'a Cache,
    search: Option<&str>,
    media: impl Iterator<Item = (&'b library::MediaId, &'b library::Media)>,
    library: &library::Library,
    sort: impl Fn(
        &(&library::MediaId, &library::Media),
        &(&library::MediaId, &library::Media),
        &library::Library,
    ) -> std::cmp::Ordering,
    limit: Option<usize>,
    now: Instant,
) -> iced::Element<'a, HomeMessage> {
    row![]
        .spacing(10.0)
        .padding(iced::Padding::new(0.0).top(20.0).bottom(20.0))
        .clip(true)
        .extend(
            search_maybe(
                media,
                search.map(|search| {
                    |&(id, _media): &(&library::MediaId, &library::Media)| {
                        sublime_fuzzy::best_match(search, &library::full_title(*id, library))
                            .map(|m| m.score())
                    }
                }),
                |a, b| sort(a, b, library),
            )
            .take(limit.unwrap_or(usize::MAX))
            .map(|(id, media)| media_card(cache, *id, media, library, now)),
        )
        .wrap()
        .into()
}

fn media_card<'a>(
    cache: &'a Cache,
    id: library::MediaId,
    media: &library::Media,
    library: &library::Library,
    now: Instant,
) -> iced::Element<'a, HomeMessage> {
    let card = cache.cache.get(&id);

    let hover_alpha = card
        .map(|card| card.hover_animation.interpolate(0.0, 1.0, now))
        .unwrap_or(1.0);

    mouse_area(
        stack![]
            .push(
                column![]
                    .spacing(5.0)
                    .width(Card::CARD_WIDTH)
                    .clip(true)
                    .push(poster_image(card))
                    .push(
                        pin(text(library::full_title(id, library))
                            .wrapping(text::Wrapping::None)
                            .size(14.0))
                        .x(card
                            .and_then(|card| card.text_animation.as_ref())
                            .map(|anim| {
                                anim.interpolate_with(|x| x, now)
                                    .clamp(card.unwrap().text_overflow, 0.0)
                            })
                            .unwrap_or(0.0)),
                    ),
            )
            .push(
                stack![]
                    .width(Card::CARD_WIDTH)
                    .height(225.0)
                    .push(
                        container(
                            row![]
                                .spacing(5.0)
                                .width(iced::Length::Fill)
                                .height(30.0)
                                .align_y(iced::Alignment::Center)
                                .push(watched_icon(
                                    library::calculate_watched(id, library).unwrap(),
                                    true,
                                    hover_alpha,
                                ))
                                .push(space::horizontal())
                                .push(collection_menu(id, library, hover_alpha))
                                .push(media_menu(id, library, hover_alpha)),
                        )
                        .width(iced::Length::Fill)
                        .height(150.0)
                        .padding(iced::Padding::new(5.0).left(10.0))
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                                iced::gradient::Linear::new(0.0)
                                    .add_stop(0.0, iced::Color::BLACK.scale_alpha(0.0))
                                    .add_stop(
                                        1.0,
                                        iced::Color::BLACK.scale_alpha(hover_alpha * 0.95),
                                    ),
                            ))),
                            ..Default::default()
                        }),
                    )
                    .push(media.video().map(move |_| {
                        icon(0xe037)
                            .color(color!(0xf0f0f0).scale_alpha(hover_alpha))
                            .size(36.0)
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill)
                            .align_x(iced::Alignment::Center)
                            .align_y(iced::Alignment::Center)
                    })),
            ),
    )
    .interaction(iced::mouse::Interaction::Pointer)
    .on_enter(HomeMessage::CardMouseEnter(id))
    .on_exit(HomeMessage::CardMouseExit(id))
    .on_press(match media {
        library::Media::Uncategorised(_)
        | library::Media::Movie(_)
        | library::Media::Episode(_) => HomeMessage::Play(id),
        library::Media::Series(_) => HomeMessage::Goto(Tab::TvShow(id)),
        library::Media::Season(_) => HomeMessage::Goto(Tab::Season(id)),
    })
    .into()
}
