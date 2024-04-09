#![allow(non_camel_case_types)]
use dashmap::DashMap;
use std::collections::HashMap;

use flume::{Receiver, Sender};
use libsqlite3_sys as ffi;
use std::sync::Arc;
use rxqlite_notification::*;






pub type ClientId = u64;

pub struct NotificationDispatcher {
    pub(crate) tx: Sender<Notification>,
    //pub rx: Receiver<Notification>,
    pub clients: Arc<DashMap<ClientId, Sender<Notification>>>,
    pub table_clients: Arc<DashMap<
      String,
      HashMap<ClientId, Sender<Notification>>
    >>,
}

impl NotificationDispatcher {
    pub fn register_client(&self) -> (ClientId, Receiver<Notification>) {
        let client_id: ClientId = self.clients.len() as _;
        let (tx, rx) = flume::unbounded();
        self.clients.insert(client_id, tx);
        (client_id, rx)
    }
    pub fn register_client_for_table(&self,table_name:& str) -> (ClientId, Receiver<Notification>) {
        let client_id: ClientId = self.clients.len() as _;
        let (tx, rx) = flume::unbounded();
        let mut entry=self.table_clients.entry(table_name.into()).or_insert(Default::default());
        entry.insert(client_id, tx);
        (client_id, rx)
    }
    pub fn unregister_client(&self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }
    pub fn unregister_client_for_table(&self, client_id: ClientId,table_name: &String) {
        if let Some(mut clients) = self.table_clients.get_mut(table_name) {
          clients.value_mut().remove(&client_id);
        }
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        let (tx, rx) = flume::unbounded::<Notification>();
        let clients: Arc<DashMap<ClientId, Sender<Notification>>> = Default::default();
        let clients2 = clients.clone();
        
        let table_clients: Arc<DashMap<String,HashMap<ClientId, Sender<Notification>>>> = Default::default();
        let table_clients2 = table_clients.clone();
        
        let _ = std::thread::spawn(move || loop {
            match rx.recv() {
                Ok(msg) => {
                    tracing::debug!("before sending notification: {:?}", msg);
                    for client in clients.iter() {
                        tracing::debug!("sending notification: {:?}", msg);
                        let _ = client.value().send(msg.clone());
                    }
                    if let Some(table)=msg.table() {
                      if let Some(clients) = table_clients.get(table) {
                        let clients = clients.value();
                        for (_,client) in clients.iter() {
                          tracing::debug!("sending notification: {:?}", msg);
                          let _ = client.send(msg.clone());
                        }
                      }
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
            table_clients: table_clients2,
        }
    }
}

pub static NOTIFICATION_DISPATCHER: state::InitCell<NotificationDispatcher> =
    state::InitCell::new();

pub use flume;
