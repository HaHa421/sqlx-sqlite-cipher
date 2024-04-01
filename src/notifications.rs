#![allow(non_camel_case_types)]
use dashmap::DashMap;
use flume::{Receiver, Sender};
use libsqlite3_sys as ffi;
use std::sync::Arc;
use rxqlite_notification::*;






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
