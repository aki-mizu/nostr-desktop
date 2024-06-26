// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::path::Path;

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Command, Element, Length};
use nostr_sdk::nostr::key::{FromSkStr, Keys};
use nostr_sdk::Client;

use crate::message::Message;
use crate::stage::auth::context::Context;
use crate::stage::auth::State;
use crate::util::dir;

#[derive(Debug, Clone)]
pub enum LoginMessage {
    SecretKeyChanged(String),
    ButtonPressed,
}

#[derive(Debug, Default)]
pub struct LoginState {
    secret_key: String,
    error: Option<String>,
}

impl LoginState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.secret_key = String::new();
        self.error = None;
    }
}

impl State for LoginState {
    fn title(&self) -> String {
        String::from("Nostr - Login")
    }

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Login(msg) = message {
            match msg {
                LoginMessage::SecretKeyChanged(secret_key) => self.secret_key = secret_key,
                LoginMessage::ButtonPressed => match Keys::from_sk_str(&self.secret_key) {
                    Ok(keys) => match Client::new_with_store(
                        &keys,
                        dir::default_dir().unwrap_or_else(|_| Path::new("./").to_path_buf()),
                    ) {
                        Ok(client) => {
                            return Command::perform(async {}, move |_| {
                                Message::LoginResult(client)
                            })
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    },
                    Err(e) => self.error = Some(e.to_string()),
                },
            }
        };

        Command::none()
    }

    fn view(&self, _ctx: &Context) -> Element<Message> {
        let text_input = text_input("Secret key", &self.secret_key)
            .on_input(|secret_key| Message::Login(LoginMessage::SecretKeyChanged(secret_key)))
            .on_submit(Message::Login(LoginMessage::ButtonPressed))
            .padding(10)
            .size(20);

        let button = button("Login")
            .padding(10)
            .on_press(Message::Login(LoginMessage::ButtonPressed));

        let content = column![
            row![text_input, button].spacing(10),
            if let Some(error) = &self.error {
                row![text(error)]
            } else {
                row![]
            }
        ]
        .spacing(20)
        .padding(20)
        .max_width(600);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

impl From<LoginState> for Box<dyn State> {
    fn from(s: LoginState) -> Box<dyn State> {
        Box::new(s)
    }
}
