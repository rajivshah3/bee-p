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

use std::{
    collections::{BinaryHeap, VecDeque},
    future::Future,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

pub(crate) struct WaitPriorityQueue<T: Ord + Eq> {
    // TODO use an RWLock ?
    inner: Mutex<(BinaryHeap<T>, VecDeque<Waker>)>,
}

impl<T: Ord + Eq> WaitPriorityQueue<T> {
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().0.is_empty()
    }
}

impl<T: Ord + Eq> Default for WaitPriorityQueue<T> {
    fn default() -> Self {
        Self {
            inner: Mutex::new((BinaryHeap::new(), VecDeque::new())),
        }
    }
}

impl<T: Ord + Eq> WaitPriorityQueue<T> {
    pub fn insert(&self, entry: T) {
        let mut inner = self.inner.lock().unwrap();

        inner.0.push(entry);
        if let Some(waker) = inner.1.pop_front() {
            Waker::wake(waker)
        }
    }

    pub fn pop(&self) -> impl Future<Output = T> + '_ {
        WaitFut(self)
    }
}

pub(crate) struct WaitFut<'a, T: Ord + Eq>(&'a WaitPriorityQueue<T>);

impl<'a, T: Ord + Eq> Future for WaitFut<'a, T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut inner = self.0.inner.lock().unwrap();

        match inner.0.pop() {
            Some(entry) => Poll::Ready(entry),
            None => {
                inner.1.push_back(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
