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

use crate::transaction::{Address, Hash, Transaction, TransactionField, Transactions};

use std::collections::HashMap;

pub struct Bundle(pub(crate) Transactions);

impl Bundle {
    // TODO TEST
    pub fn get(&self, index: usize) -> Option<&Transaction> {
        self.0.get(index)
    }

    // TODO TEST
    pub fn len(&self) -> usize {
        self.0.len()
    }

    // TODO TEST
    pub fn hash(&self) -> &Hash {
        // Safe to unwrap because empty bundles can't be built
        self.get(0).unwrap().bundle()
    }

    // TODO TEST
    pub fn tail(&self) -> &Transaction {
        // Safe to unwrap because empty bundles can't be built
        self.get(0).unwrap()
    }

    // TODO TEST
    pub fn head(&self) -> &Transaction {
        // Safe to unwrap because empty bundles can't be built
        self.get(self.len() - 1).unwrap()
    }

    // TODO TEST
    pub fn trunk(&self) -> &Hash {
        self.head().trunk()
    }

    // TODO TEST
    pub fn branch(&self) -> &Hash {
        self.head().branch()
    }

    // TODO TEST
    pub fn ledger_diff(&self) -> HashMap<Address, i64> {
        let mut diff = HashMap::new();

        for transaction in self {
            if *transaction.value.to_inner() != 0 {
                *diff.entry(transaction.address().clone()).or_insert(0) += *transaction.value.to_inner();
            }
        }

        diff
    }
}

impl IntoIterator for Bundle {
    type Item = Transaction;
    type IntoIter = std::vec::IntoIter<Transaction>;

    // TODO TEST
    fn into_iter(self) -> Self::IntoIter {
        (self.0).0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Bundle {
    type Item = &'a Transaction;
    type IntoIter = std::slice::Iter<'a, Transaction>;

    // TODO TEST
    fn into_iter(self) -> Self::IntoIter {
        (&(self.0).0).iter()
    }
}

impl std::ops::Index<usize> for Bundle {
    type Output = Transaction;

    // TODO TEST
    fn index(&self, index: usize) -> &Self::Output {
        // Unwrap because index is expected to panic if out of range
        self.get(index).unwrap()
    }
}

#[cfg(test)]
mod tests {}
