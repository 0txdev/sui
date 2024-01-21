// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::mock_account::Account;
pub use move_tx_generator::MoveTxGenerator;
pub use non_move_tx_generator::NonMoveTxGenerator;
pub use root_object_create_tx_generator::RootObjectCreateTxGenerator;
use sui_types::transaction::Transaction;

pub mod move_tx_generator;
pub mod non_move_tx_generator;
pub mod root_object_create_tx_generator;

pub trait TxGenerator: Send + Sync {
    /// Given an account that contains a sender address, a keypair for that address,
    /// and a list of gas objects owned by this address, generate a single transaction.
    fn generate_tx(&self, account: Account) -> Transaction;

    fn name(&self) -> &'static str;
}
