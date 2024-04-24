// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use async_stream::stream;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::advanced::Hasher;
use iced::Subscription;
use iced_futures::BoxStream;
use nostr_sdk::event::kind;
use nostr_sdk::nostr::Event;
use nostr_sdk::{Client, Filter, Kind, NostrDatabaseExt, PublicKey, RelayPoolNotification, SubscribeOptions};
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
            if let Ok(signer) = client.signer().await {
                if let Ok(public_key) = signer.public_key().await {
                    for (relay_url, relay) in client.relays().await {
                        let contacts: Vec<PublicKey> = client
                        .database()
                        .contacts_public_keys(public_key)
                        .await
                        .unwrap_or_default();
                        let base_filter = Filter::new().kinds([
                            Kind::Metadata,
                            Kind::TextNote,
                            Kind::Repost,
                            Kind::Reaction,
                        ]).author(public_key);
                        let filters: Vec<Filter> = vec![base_filter];
                        if !contacts.is_empty() {
                            filters.push(Filter::new().authors(contacts));
                        }
                        if let Err(e) = relay
                            .subscribe(
                                filters,
                                SubscribeOptions::default(),
                            )
                            .await
                        {
                            panic!("Impossible to start sync thread: {}", e);
                        }
                    }
                }
            }
        });

        let client = self.client.clone();
        let join = tokio::task::spawn(async move {
            let mut notifications = client.notifications();
            while let Ok(notification) = notifications.recv().await {
                match notification {
                    RelayPoolNotification::Event{ event, ..} => {
                        // TODO: Send desktop notification
                        sender.send(*event).ok();
                    }
                    RelayPoolNotification::Shutdown => break,
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
