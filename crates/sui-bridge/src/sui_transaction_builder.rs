// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, str::FromStr};

use fastcrypto::traits::ToFromBytes;
use move_core_types::ident_str;
use once_cell::sync::OnceCell;
use sui_types::gas_coin::GAS;
use sui_types::BRIDGE_PACKAGE_ID;
use sui_types::{
    base_types::{ObjectRef, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{ObjectArg, TransactionData},
    TypeTag,
};

use crate::{
    error::{BridgeError, BridgeResult},
    types::{BridgeAction, TokenId, VerifiedCertifiedBridgeAction},
};

// TODO: how do we generalize this thing more?
pub fn get_sui_token_type_tag(token_id: TokenId) -> TypeTag {
    static TYPE_TAGS: OnceCell<HashMap<TokenId, TypeTag>> = OnceCell::new();
    let type_tags = TYPE_TAGS.get_or_init(|| {
        let package_id = BRIDGE_PACKAGE_ID;
        let mut type_tags = HashMap::new();
        type_tags.insert(TokenId::Sui, GAS::type_tag());
        type_tags.insert(
            TokenId::BTC,
            TypeTag::from_str(&format!("{:?}::btc::BTC", package_id)).unwrap(),
        );
        type_tags.insert(
            TokenId::ETH,
            TypeTag::from_str(&format!("{:?}::eth::ETH", package_id)).unwrap(),
        );
        type_tags.insert(
            TokenId::USDC,
            TypeTag::from_str(&format!("{:?}::usdc::USDC", package_id)).unwrap(),
        );
        type_tags.insert(
            TokenId::USDT,
            TypeTag::from_str(&format!("{:?}::usdt::USDT", package_id)).unwrap(),
        );
        type_tags
    });
    type_tags.get(&token_id).unwrap().clone()
}

// TODO: pass in gas price
pub fn build_transaction(
    client_address: SuiAddress,
    gas_object_ref: &ObjectRef,
    action: VerifiedCertifiedBridgeAction,
    bridge_object_arg: ObjectArg,
) -> BridgeResult<TransactionData> {
    match action.data() {
        BridgeAction::EthToSuiBridgeAction(_) => build_token_bridge_approve_transaction(
            client_address,
            gas_object_ref,
            action,
            true,
            bridge_object_arg,
        ),
        BridgeAction::SuiToEthBridgeAction(_) => build_token_bridge_approve_transaction(
            client_address,
            gas_object_ref,
            action,
            false,
            bridge_object_arg,
        ),
        BridgeAction::BlocklistCommitteeAction(_) => {
            // TODO: handle this case
            unimplemented!()
        }
        BridgeAction::EmergencyAction(_) => {
            // TODO: handle this case
            unimplemented!()
        }
        BridgeAction::LimitUpdateAction(_) => {
            // TODO: handle this case
            unimplemented!()
        }
        BridgeAction::AssetPriceUpdateAction(_) => {
            // TODO: handle this case
            unimplemented!()
        }
        BridgeAction::EvmContractUpgradeAction(_) => {
            // TODO: handle this case
            unimplemented!()
        }
    }
}

// TODO: pass in gas price
fn build_token_bridge_approve_transaction(
    client_address: SuiAddress,
    gas_object_ref: &ObjectRef,
    action: VerifiedCertifiedBridgeAction,
    claim: bool,
    bridge_object_arg: ObjectArg,
) -> BridgeResult<TransactionData> {
    let (bridge_action, sigs) = action.into_inner().into_data_and_sig();
    let mut builder = ProgrammableTransactionBuilder::new();

    let (source_chain, seq_num, sender, target_chain, target, token_type, amount) =
        match bridge_action {
            BridgeAction::SuiToEthBridgeAction(a) => {
                let bridge_event = a.sui_bridge_event;
                (
                    bridge_event.sui_chain_id,
                    bridge_event.nonce,
                    bridge_event.sui_address.to_vec(),
                    bridge_event.eth_chain_id,
                    bridge_event.eth_address.to_fixed_bytes().to_vec(),
                    bridge_event.token_id,
                    bridge_event.amount,
                )
            }
            BridgeAction::EthToSuiBridgeAction(a) => {
                let bridge_event = a.eth_bridge_event;
                (
                    bridge_event.eth_chain_id,
                    bridge_event.nonce,
                    bridge_event.eth_address.to_fixed_bytes().to_vec(),
                    bridge_event.sui_chain_id,
                    bridge_event.sui_address.to_vec(),
                    bridge_event.token_id,
                    bridge_event.amount,
                )
            }
            _ => unreachable!(),
        };

    let source_chain = builder.pure(source_chain as u8).unwrap();
    let seq_num = builder.pure(seq_num).unwrap();
    let sender = builder.pure(sender.clone()).map_err(|e| {
        BridgeError::BridgeSerializationError(format!(
            "Failed to serialize sender: {:?}. Err: {:?}",
            sender, e
        ))
    })?;
    let target_chain = builder.pure(target_chain as u8).unwrap();
    let target = builder.pure(target.clone()).map_err(|e| {
        BridgeError::BridgeSerializationError(format!(
            "Failed to serialize target: {:?}. Err: {:?}",
            target, e
        ))
    })?;
    let arg_token_type = builder.pure(token_type as u8).unwrap();
    let amount = builder.pure(amount).unwrap();

    let arg_msg = builder.programmable_move_call(
        BRIDGE_PACKAGE_ID,
        ident_str!("message").to_owned(),
        ident_str!("create_token_bridge_message").to_owned(),
        vec![],
        vec![
            source_chain,
            seq_num,
            sender,
            target_chain,
            target,
            arg_token_type,
            amount,
        ],
    );

    // Unwrap: this should not fail
    let arg_bridge = builder.obj(bridge_object_arg).unwrap();

    let mut sig_bytes = vec![];
    for (_, sig) in sigs.signatures {
        sig_bytes.push(sig.as_bytes().to_vec());
    }
    let arg_signatures = builder.pure(sig_bytes.clone()).map_err(|e| {
        BridgeError::BridgeSerializationError(format!(
            "Failed to serialize signatures: {:?}. Err: {:?}",
            sig_bytes, e
        ))
    })?;

    builder.programmable_move_call(
        BRIDGE_PACKAGE_ID,
        ident_str!("bridge").to_owned(),
        ident_str!("approve_bridge_message").to_owned(),
        vec![],
        vec![arg_bridge, arg_msg, arg_signatures],
    );

    if claim {
        builder.programmable_move_call(
            BRIDGE_PACKAGE_ID,
            ident_str!("bridge").to_owned(),
            ident_str!("claim_and_transfer_token").to_owned(),
            vec![get_sui_token_type_tag(token_type)],
            vec![arg_bridge, source_chain, seq_num],
        );
    }

    let pt = builder.finish();

    Ok(TransactionData::new_programmable(
        client_address,
        vec![*gas_object_ref],
        pt,
        15_000_000,
        // TODO: use reference gas price
        1500,
    ))
}
