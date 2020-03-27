use crate::address::Address;
use crate::endpoint::{role::Role, whitelist, whitelist::WhiteList};
use crate::errors::Result;
use crate::events::EventPublisher as Notifier;
use crate::shutdown::ShutdownListener as Shutdown;

use super::connection::TcpConnection;
use super::spawn_connection_workers;

use async_std::net::TcpListener;
use futures::prelude::*;
use futures::select;
use log::*;

pub(crate) struct TcpWorker {
    binding_addr: Address,
    notifier: Notifier,
    shutdown: Shutdown,
}

impl TcpWorker {
    pub fn new(binding_addr: Address, notifier: Notifier, shutdown: Shutdown) -> Self {
        Self {
            binding_addr,
            notifier,
            shutdown,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        debug!("[TCP  ] Starting TCP worker...");

        let listener = TcpListener::bind(*self.binding_addr).await?;

        debug!("[TCP  ] Accepting connections on {}.", listener.local_addr()?);

        let mut incoming = listener.incoming().fuse();
        let shutdown = &mut self.shutdown;

        loop {
            select! {
                stream = incoming.next() => {
                    if let Some(stream) = stream {
                        match stream {
                            Ok(stream) => {

                                let conn = match TcpConnection::new(stream, Role::Server) {
                                    Ok(conn) => conn,
                                    Err(e) => {
                                        error!["TCP  ] Error creating TCP connection (Stream immediatedly aborted?)."];
                                        error!["TCP  ] Error was: {:?}.", e];
                                        continue;
                                    }
                                };

                                let whitelist = whitelist::get();

                                // Update IP addresses if necessary
                                // whitelist.refresh().await;

                                // Immediatedly drop stream, if it's associated IP address isn't whitelisted
                                if !whitelist.contains_address(&conn.remote_addr.ip()) {
                                    warn!("[TCP  ] Contacted by unknown IP address '{}'.", &conn.remote_addr.ip());
                                    warn!("[TCP  ] Connection disallowed.");
                                    continue;
                                }

                                debug!(
                                    "[TCP  ] Sucessfully established connection to {} (as {}).",
                                    conn.remote_addr,
                                    Role::Server
                                );

                                match spawn_connection_workers(conn, self.notifier.clone()).await {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                            }
                            Err(e) => {
                                error!("[TCP  ] Connection attempt failed (Endpoint offline?).");
                                error!("[TCP  ] Error was: {:?}.", e);
                            },
                        }
                    } else {
                        break;
                    }
                },
                shutdown = shutdown.fuse() => {
                    break;
                }
            }
        }

        debug!("[TCP  ] Stopped TCP worker.");
        Ok(())
    }
}
