#![allow(non_camel_case_types)]
use dashmap::DashMap;
use flume::{Receiver, Sender};
use libsqlite3_sys as ffi;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

//https://github.com/rusqlite/rusqlite/blob/b41bd805710149ebfaed577dfedb464338e2ca97/src/hooks.rs#L17

/// Action Codes
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
#[non_exhaustive]
#[allow(clippy::upper_case_acronyms)]
pub enum Action {
    /// Unsupported / unexpected action
    UNKNOWN = -1,
    /// DELETE command
    SQLITE_DELETE = ffi::SQLITE_DELETE,
    /// INSERT command
    SQLITE_INSERT = ffi::SQLITE_INSERT,
    /// UPDATE command
    SQLITE_UPDATE = ffi::SQLITE_UPDATE,
}

impl From<i32> for Action {
    #[inline]
    fn from(code: i32) -> Action {
        match code {
            ffi::SQLITE_DELETE => Action::SQLITE_DELETE,
            ffi::SQLITE_INSERT => Action::SQLITE_INSERT,
            ffi::SQLITE_UPDATE => Action::SQLITE_UPDATE,
            _ => Action::UNKNOWN,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    Update {
        action: Action,
        database: String,
        table: String,
        row_id: i64,
    },
}

pub type ClientId = u64;

pub struct NotificationDispatcher {
    pub(crate) tx: Sender<Notification>,
    //pub rx: Receiver<Notification>,
    pub clients: Arc<DashMap<ClientId, Sender<Notification>>>,
}

impl NotificationDispatcher {
    pub fn register_client(&self) -> (ClientId, Receiver<Notification>) {
        let client_id: ClientId = self.clients.len() as _;
        let (tx, rx) = flume::unbounded();
        self.clients.insert(client_id, tx);
        (client_id, rx)
    }
    pub fn unregister_client(&self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        let (tx, rx) = flume::unbounded::<Notification>();
        let clients: Arc<DashMap<ClientId, Sender<Notification>>> = Default::default();
        let clients2 = clients.clone();
        let _ = std::thread::spawn(move || loop {
            match rx.recv() {
                Ok(msg) => {
                    tracing::debug!("before sending notification: {:?}", msg);
                    for client in clients.iter() {
                        tracing::debug!("sending notification: {:?}", msg);
                        let _ = client.value().send(msg.clone());
                    }
                }
                Err(err) => {
                    tracing::error!("{}", err);
                    break;
                }
            }
        });
        Self {
            tx,
            clients: clients2,
        }
    }
}

pub static NOTIFICATION_DISPATCHER: state::InitCell<NotificationDispatcher> =
    state::InitCell::new();

pub use flume;
