// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use async_stream::stream;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::advanced::Hasher;
use iced::Subscription;
use iced_futures::BoxStream;
use nostr_sdk::nostr::Event;
use nostr_sdk::{Client, RelayPoolNotifications};
use tokio::sync::mpsc;

use crate::RUNTIME;

pub struct NostrSync {
    client: Client,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl Recipe for NostrSync {
    type Output = Event;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(mut self: Box<Self>, _input: EventStream) -> BoxStream<Self::Output> {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let client = self.client.clone();
        RUNTIME.block_on(async move {
            if let Err(e) = client.sync().await {
                panic!("Impossible to start sync thread: {}", e);
            }
        });

        let client = self.client.clone();
        let join = tokio::task::spawn(async move {
            let mut notifications = client.notifications();
            while let Ok(notification) = notifications.recv().await {
                match notification {
                    RelayPoolNotifications::ReceivedEvent(event) => {
                        // TODO: Send desktop notification
                        sender.send(event).ok();
                    }
                    RelayPoolNotifications::Shutdown => break,
                    _ => (),
                }
            }
            log::debug!("Exited from notification thread");
        });
        self.join = Some(join);
        let stream = stream! {
            while let Some(item) = receiver.recv().await {
                yield item;
            }
        };
        Box::pin(stream)
    }
}

impl NostrSync {
    pub fn subscription(client: Client) -> Subscription<Event> {
        Subscription::from_recipe(Self { client, join: None })
    }
}
