use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    num::NonZeroU8,
    sync::{Arc, OnceLock},
    time::Duration,
};

use futures::lock::Mutex;
use rfs::{interfaces::FileUpdate, middleware::TransmissionProtocol, ser_de};
use tokio::net::UdpSocket;

use crate::server::FileUpdateCallback;

// lazy_static! {
//     pub static ref FILE_UPDATE_CALLBACKS: Arc<Mutex<HashMap<String, Vec<FileUpdateCallback>>>> =
//         { Arc::new(Mutex::new(HashMap::new())) };
// }

/// Callbacks for file updates.
pub static FILE_UPDATE_CALLBACKS: OnceLock<Arc<Mutex<RegisteredFileUpdates>>> = OnceLock::new();

#[derive(Debug)]
pub struct RegisteredFileUpdates {
    /// Server address. The port will be determined by the OS.
    pub bind_addr: Ipv4Addr,
    /// Registered file callbacks
    pub lookup: HashMap<String, Vec<FileUpdateCallback>>,
    /// Transmission protocol, same as server.
    pub proto: Arc<dyn TransmissionProtocol + Send + Sync>,

    pub timeout: Duration,
    pub retries: u8,
}

impl RegisteredFileUpdates {
    /// Searches for the file update callbacks and triggers them, if any.
    ///
    /// Returns the number of callbacks triggered.
    pub async fn trigger_file_update(
        &mut self,
        path: &str,
        contents: FileUpdate,
    ) -> Option<NonZeroU8> {
        log::debug!("checking for file update callbacks for {}", path);

        let callbacks = self.lookup.remove(path)?;

        let num_targets = callbacks.len();

        let sock = Arc::new(
            UdpSocket::bind(SocketAddrV4::new(self.bind_addr, 0))
                .await
                .ok()?,
        );

        let ser_payload = Arc::new(ser_de::serialize(&contents).ok()?);

        let handles = callbacks.iter().map(|cb| {
            let proto = self.proto.clone();
            let sock_clone = sock.clone();
            let pl = ser_payload.clone();
            let ad = cb.addr;
            let to = self.timeout.clone();
            let rt = self.retries.clone();

            (
                tokio::spawn(async move { proto.send_bytes(&sock_clone, ad, &pl, to, rt).await }),
                ad,
            )
        });

        for (handle, addr) in handles {
            handle.await.inspect_err(|e| {
                log::error!("error sending file update to {}: {:?}", addr, e);
            });
        }

        NonZeroU8::new(num_targets as u8)
    }
}
