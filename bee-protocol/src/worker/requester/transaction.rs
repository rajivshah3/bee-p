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

use crate::{message::TransactionRequest, milestone::MilestoneIndex, protocol::Protocol, worker::SenderWorker};

use bee_bundle::Hash;
use bee_tangle::tangle;
use bee_ternary::T5B1Buf;

use std::cmp::Ordering;

use bytemuck::cast_slice;
use futures::{channel::oneshot, future::FutureExt, select};
use log::info;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

#[derive(Eq, PartialEq)]
pub(crate) struct TransactionRequesterWorkerEntry(pub(crate) Hash, pub(crate) MilestoneIndex);

// TODO check that this is the right order
impl PartialOrd for TransactionRequesterWorkerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for TransactionRequesterWorkerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

pub(crate) struct TransactionRequesterWorker {
    rng: Pcg32,
}

impl TransactionRequesterWorker {
    pub(crate) fn new() -> Self {
        Self {
            rng: Pcg32::from_entropy(),
        }
    }

    async fn process_request(&mut self, hash: Hash, index: MilestoneIndex) {
        if Protocol::get().peer_manager.handshaked_peers.is_empty() {
            return;
        }

        // TODO check that neighbor may have the tx (by the index)
        Protocol::get().requested.insert(hash, index);

        match Protocol::get().peer_manager.handshaked_peers.iter().nth(
            self.rng
                .gen_range(0, Protocol::get().peer_manager.handshaked_peers.len()),
        ) {
            Some(entry) => {
                SenderWorker::<TransactionRequest>::send(
                    entry.key(),
                    TransactionRequest::new(cast_slice(hash.as_trits().encode::<T5B1Buf>().as_i8_slice())),
                )
                .await;
            }
            None => {}
        }
    }

    pub(crate) async fn run(mut self, shutdown: oneshot::Receiver<()>) {
        info!("[TransactionRequesterWorker ] Running.");

        let mut shutdown_fused = shutdown.fuse();

        loop {
            select! {
                // TODO impl fused stream
                entry = Protocol::get().transaction_requester_worker.0.pop().fuse() => {
                    if let TransactionRequesterWorkerEntry(hash, index) = entry {
                        if !tangle().is_solid_entry_point(&hash) && !tangle().contains_transaction(&hash) {
                            self.process_request(hash, index).await;
                        }
                    }
                },
                _ = shutdown_fused => {
                    break;
                }
            }
        }

        info!("[TransactionRequesterWorker ] Stopped.");
    }
}
