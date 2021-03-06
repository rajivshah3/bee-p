// Copyright 2020 IOTA Stiftung
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
// the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
// an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and limitations under the License.

use crate::{
    message::{uncompress_transaction_bytes, TransactionBroadcast},
    protocol::Protocol,
    worker::transaction::TinyHashCache,
};

use bee_bundle::{Hash, Transaction, TransactionField};
use bee_crypto::{CurlP81, Sponge};
use bee_network::EndpointId;
use bee_tangle::tangle;
use bee_ternary::{T1B1Buf, T5B1Buf, Trits, T5B1};

use futures::{
    channel::{mpsc, oneshot},
    future::FutureExt,
    select,
    stream::StreamExt,
    SinkExt,
};
use log::{debug, error, info, warn};

pub(crate) struct TransactionWorkerEvent {
    pub(crate) from: EndpointId,
    pub(crate) transaction_broadcast: TransactionBroadcast,
}

pub(crate) struct TransactionWorker {
    cache: TinyHashCache,
    curl: CurlP81,
}

impl TransactionWorker {
    pub(crate) fn new(cache_size: usize) -> Self {
        Self {
            cache: TinyHashCache::new(cache_size),
            curl: CurlP81::new(),
        }
    }

    pub(crate) async fn run(
        mut self,
        receiver: mpsc::Receiver<TransactionWorkerEvent>,
        shutdown: oneshot::Receiver<()>,
        mut milestone_validator_worker_tx: mpsc::Sender<Hash>,
    ) {
        info!("[TransactionWorker ] Running.");

        let mut receiver_fused = receiver.fuse();
        let mut shutdown_fused = shutdown.fuse();

        loop {
            select! {
                event = receiver_fused.next() => {
                    if let Some(TransactionWorkerEvent{from, transaction_broadcast}) = event {
                        self.process_transaction_brodcast(from, transaction_broadcast, &mut milestone_validator_worker_tx).await;
                    }
                },
                _ = shutdown_fused => break
            }
        }

        info!("[TransactionWorker ] Stopped.");
    }

    async fn process_transaction_brodcast(
        &mut self,
        from: EndpointId,
        transaction_broadcast: TransactionBroadcast,
        milestone_validator_worker_tx: &mut mpsc::Sender<Hash>,
    ) {
        debug!("[TransactionWorker ] Processing received data...");

        if !self.cache.insert(&transaction_broadcast.transaction) {
            debug!("[TransactionWorker ] Data already received.");
            return;
        }

        // convert received transaction bytes into T1B1 buffer
        let transaction_buf = {
            let u8_t5b1_buf = uncompress_transaction_bytes(&transaction_broadcast.transaction);

            // transform [u8] to &[i8]
            let i8_t5b1_slice = unsafe { &*(&u8_t5b1_buf as *const [u8] as *const [i8]) };

            // get T5B1 trits
            let t5b1_trits_result = Trits::<T5B1>::try_from_raw(i8_t5b1_slice, i8_t5b1_slice.len() * 5 - 1);

            match t5b1_trits_result {
                Ok(t5b1_trits) => {
                    // get T5B1 trit_buf
                    let t5b1_trit_buf = t5b1_trits.to_buf::<T5B1Buf>();

                    // get T1B1 trit_buf from TB51 trit_buf
                    t5b1_trit_buf.encode::<T1B1Buf>()
                }
                Err(_) => {
                    warn!("[TransactionWorker ] Can not decode T5B1 from received data.");
                    return;
                }
            }
        };

        // build transaction
        let transaction = match Transaction::from_trits(&transaction_buf) {
            Ok(transaction) => transaction,
            Err(e) => {
                warn!(
                    "[TransactionWorker ] Can not build transaction from received data: {:?}",
                    e
                );
                return;
            }
        };

        // calculate transaction hash
        let hash = Hash::from_inner_unchecked(self.curl.digest(&transaction_buf).unwrap());

        if hash.weight() < Protocol::get().config.mwm {
            debug!("[TransactionWorker ] Insufficient weight magnitude: {}.", hash.weight());
            return;
        }

        // store transaction
        match tangle().insert_transaction(transaction, hash).await {
            Some(transaction) => {
                if !tangle().is_synced() && Protocol::get().requested.is_empty() {
                    Protocol::trigger_milestone_solidification().await;
                }
                match Protocol::get().requested.remove(&hash) {
                    Some((hash, index)) => {
                        Protocol::trigger_transaction_solidification(hash, index).await;
                    }
                    None => Protocol::broadcast_transaction_message(Some(from), transaction_broadcast).await,
                };

                if transaction.address().eq(&Protocol::get().config.coordinator.public_key)
                    || transaction.address().eq(&Protocol::get().config.workers.null_address)
                {
                    let tail = {
                        if transaction.is_tail() {
                            Some(hash)
                        } else {
                            let chain =
                                tangle().trunk_walk_approvers(hash, |tx_ref| tx_ref.bundle() == transaction.bundle());
                            match chain.last() {
                                Some((tx_ref, hash)) => {
                                    if tx_ref.is_tail() {
                                        Some(*hash)
                                    } else {
                                        None
                                    }
                                }
                                None => None,
                            }
                        }
                    };

                    if let Some(tail) = tail {
                        if let Err(e) = milestone_validator_worker_tx.send(tail).await {
                            error!(
                                "[TransactionWorker ] Sending tail to milestone validation failed: {:?}.",
                                e
                            );
                        }
                    };
                }
            }
            None => {
                debug!(
                    "[TransactionWorker ] Transaction {} already present in the tangle.",
                    &hash
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::ProtocolConfig;

    use bee_network::{NetworkConfig, Url};

    use async_std::task::{block_on, spawn};
    use futures::sink::SinkExt;

    #[test]
    fn test_tx_worker_with_compressed_buffer() {
        bee_tangle::init();

        // build network
        let network_config = NetworkConfig::build().finish();
        let (network, _shutdown, _receiver) = bee_network::init(network_config);

        // init protocol
        let protocol_config = ProtocolConfig::build().finish();
        block_on(Protocol::init(protocol_config, network));

        assert_eq!(tangle().size(), 0);

        let (transaction_worker_sender, transaction_worker_receiver) = mpsc::channel(1000);
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let (milestone_validator_worker_sender, _milestone_validator_worker_receiver) = mpsc::channel(1000);

        let mut transaction_worker_sender_clone = transaction_worker_sender;

        spawn(async move {
            let tx: [u8; 1024] = [0; 1024];
            let message = TransactionBroadcast::new(&tx);
            let epid: EndpointId = block_on(Url::from_url_str("tcp://[::1]:16000")).unwrap().into();
            let event = TransactionWorkerEvent {
                from: epid,
                transaction_broadcast: message,
            };
            transaction_worker_sender_clone.send(event).await.unwrap();
        });

        spawn(async move {
            use async_std::task;
            use std::time::Duration;
            task::sleep(Duration::from_secs(1)).await;
            shutdown_sender.send(()).unwrap();
        });

        block_on(TransactionWorker::new(10000).run(
            transaction_worker_receiver,
            shutdown_receiver,
            milestone_validator_worker_sender,
        ));

        assert_eq!(tangle().size(), 1);
        assert_eq!(tangle().contains_transaction(&Hash::zeros()), true);
    }
}
