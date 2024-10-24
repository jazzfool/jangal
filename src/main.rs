#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod library;
mod settings;
mod ui;

use iced::{color, window};
use ui::app::App;

fn main() -> iced::Result {
    /*
    env_logger::init_from_env(env_logger::Env::default().default_filter_or(
        if cfg!(debug_assertions) {
            "info"
        } else {
            "error"
        },
    ));
    */

    iced::application("Jangal", App::update, App::view)
        .theme(|_| {
            iced::Theme::custom(
                "Jangal".into(),
                iced::theme::Palette {
                    background: color!(0x111111),
                    text: color!(0xf0f0f0),
                    primary: color!(0x007aff),
                    success: color!(0x50cc4e),
                    danger: color!(0xcc5d4e),
                },
            )
        })
        .settings(iced::Settings {
            fonts: vec![
                include_bytes!("ui/resources/WorkSans-Regular.ttf").into(),
                include_bytes!("ui/resources/WorkSans-Bold.ttf").into(),
                include_bytes!("ui/resources/WorkSans-Italic.ttf").into(),
                include_bytes!("ui/resources/WorkSans-BoldItalic.ttf").into(),
                include_bytes!("ui/resources/Lexend-Regular.ttf").into(),
                include_bytes!("ui/resources/IBMPlexMono-Regular.ttf").into(),
                include_bytes!("ui/resources/MaterialSymbolsSharp_Filled-Regular.ttf").into(),
            ],
            default_font: ui::SANS_FONT,
            default_text_size: 16.0.into(),
            ..Default::default()
        })
        .window(window::Settings {
            icon: Some(
                window::icon::from_file_data(include_bytes!("ui/resources/icon.png"), None)
                    .unwrap(),
            ),
            exit_on_close_request: false,
            ..Default::default()
        })
        .subscription(App::subscription)
        .run_with(App::new)
}
