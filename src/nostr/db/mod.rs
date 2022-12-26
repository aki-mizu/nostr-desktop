// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::path::Path;
use std::sync::Arc;

use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::{Contact, Sha256Hash};
use nostr_sdk::Result;

pub mod model;
mod rocksdb;
mod util;

use self::model::{Profile, TextNote};
use self::rocksdb::{
    BoundColumnFamily, Direction, IteratorMode, Store as RocksStore, WriteBatch,
    WriteSerializedBatch,
};

#[derive(Debug, Clone)]
pub struct Store {
    db: RocksStore,
}

//const EVENT_CF: &str = "event";
const AUTHOR_CF: &str = "author";
const CONTACT_CF: &str = "contact";
const PROFILE_CF: &str = "profile";
const CHAT_CF: &str = "chat";
const CHANNEL_CF: &str = "channel";
const TEXTNOTE_CF: &str = "textnote";
const TEXTNOTE_BY_TIMESTAMP: &str = "textnotebytimestamp";

const COLUMN_FAMILIES: &[&str] = &[
    //EVENT_CF,
    AUTHOR_CF,
    CONTACT_CF,
    PROFILE_CF,
    CHAT_CF,
    CHANNEL_CF,
    TEXTNOTE_CF,
    TEXTNOTE_BY_TIMESTAMP,
];

impl Store {
    pub fn open<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            db: RocksStore::open(path, COLUMN_FAMILIES)?,
        })
    }

    /* fn event_cf(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(EVENT_CF)
    } */

    fn author_cf(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(AUTHOR_CF)
    }

    fn profile_cf(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(PROFILE_CF)
    }

    fn contact_cf(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(CONTACT_CF)
    }

    fn textnote_cf(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(TEXTNOTE_CF)
    }

    fn textnote_by_timestamp(&self) -> Arc<BoundColumnFamily> {
        self.db.cf_handle(TEXTNOTE_BY_TIMESTAMP)
    }

    /* pub fn save_event(&self, event: NostrEvent) -> Result<()> {
        Ok(self
            .db
            .put_serialized(self.event_cf(), util::hash_prefix(event.id), &Event::from(event))?)
    }

    pub fn save_events(&self, events: Vec<NostrEvent>) -> Result<()> {
        let mut batch = WriteBatch::default();

        for event in events.into_iter() {
            batch.put_serialized(self.event_cf(), util::hash_prefix(event.id), &Event::from(event))?;
        }

        Ok(self.db.write(batch)?)
    }

    pub fn get_events(&self) -> Result<Vec<Event>> {
        Ok(self.db.iterator_value_serialized(self.event_cf())?)
    } */

    pub fn set_profile(&self, public_key: XOnlyPublicKey, profile: Profile) -> Result<()> {
        Ok(self.db.put(
            self.profile_cf(),
            self.db.serialize(public_key)?,
            self.db.serialize(profile)?,
        )?)
    }

    pub fn get_profile(&self, public_key: XOnlyPublicKey) -> Result<Profile> {
        Ok(self
            .db
            .get_deserialized(self.profile_cf(), self.db.serialize(public_key)?)?)
    }

    pub fn set_contacts(&self, list: Vec<Contact>) -> Result<()> {
        let mut batch = WriteBatch::default();

        for contact in list.iter() {
            batch.put_serialized(self.contact_cf(), self.db.serialize(contact.pk)?, contact)?;
        }

        Ok(self.db.write(batch)?)
    }

    pub fn get_contacts(&self) -> Result<Vec<Contact>> {
        let mut contacts = self.db.iterator_value_serialized(self.contact_cf())?;
        contacts.sort();
        Ok(contacts)
    }

    pub fn set_author(&self, public_key: XOnlyPublicKey) -> Result<()> {
        Ok(self
            .db
            .put(self.author_cf(), self.db.serialize(public_key)?, b"")?)
    }

    pub fn set_authors(&self, authors: Vec<XOnlyPublicKey>) -> Result<()> {
        let mut batch = WriteBatch::default();

        for author in authors.iter() {
            batch.put_cf(&self.author_cf(), self.db.serialize(author)?, b"");
        }

        Ok(self.db.write(batch)?)
    }

    pub fn get_authors(&self) -> Result<Vec<XOnlyPublicKey>> {
        Ok(self.db.iterator_key_serialized(self.author_cf())?)
    }

    pub fn set_textnote(&self, event_id: Sha256Hash, note: TextNote) -> Result<()> {
        let mut batch = WriteBatch::default();

        let event_id_prefix = util::hash_prefix(event_id);

        let timestamp = note.timestamp.to_be_bytes();
        if !self
            .db
            .key_may_exist(self.textnote_by_timestamp(), timestamp)
        {
            batch.put_cf(&self.textnote_by_timestamp(), timestamp, event_id_prefix);
        }

        if !self.db.key_may_exist(self.textnote_cf(), event_id_prefix) {
            batch.put_cf(
                &self.textnote_cf(),
                event_id_prefix,
                self.db.serialize(note)?,
            );
        }

        Ok(self.db.write(batch)?)
    }

    pub fn get_textnote(&self, event_id: Sha256Hash) -> Result<TextNote> {
        Ok(self
            .db
            .get_deserialized(self.textnote_cf(), util::hash_prefix(event_id))?)
    }

    pub fn get_textnotes_with_limit(&self, limit: usize) -> Vec<TextNote> {
        let ids: Vec<Vec<u8>> = self
            .db
            .iterator_with_mode(self.textnote_by_timestamp(), IteratorMode::End)
            .take(limit)
            .map(|(_, v)| v)
            .collect();

        self.db
            .multi_get(self.textnote_cf(), ids)
            .flatten()
            .flatten()
            .filter_map(|slice| self.db.deserialize(&slice).ok())
            .collect()
    }

    pub fn get_textnotes_from_timestamp(
        &self,
        timestamp: u64,
        direction: Direction,
        limit: usize,
    ) -> Vec<TextNote> {
        let ids: Vec<Vec<u8>> = self
            .db
            .iterator_with_mode(
                self.textnote_by_timestamp(),
                IteratorMode::From(&timestamp.to_be_bytes(), direction),
            )
            .take(limit)
            .map(|(_, v)| v)
            .collect();

        self.db
            .multi_get(self.textnote_cf(), ids)
            .flatten()
            .flatten()
            .filter_map(|slice| self.db.deserialize(&slice).ok())
            .collect()
    }

    pub fn flush(&self) {
        self.db.flush();
    }
}
