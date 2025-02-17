use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_program::pubkey::Pubkey;
use solana_sdk::account::{Account, ReadableAccount};
use crate::deserialize::discriminator::partition_discriminator_from_data;
use crate::deserialize::{AnchorDeserializer, IdlWithDiscriminators};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeserializedAccount {
    pub ui_account: UiAccount,
    pub program_name: String,
    pub account_type: String,
    pub deserialized: Value,
}

impl IdlWithDiscriminators {
    pub fn try_deserialize_account(&self, account: &Account) -> Result<(String, Value)> {
        let mut idl_type_defs = self.types.clone();
        idl_type_defs.extend_from_slice(&self.accounts);
        let data = account.data();
        let (discriminator, data) = partition_discriminator_from_data(data);
        let type_def = self
            .account_definitions
            .get(&discriminator)
            .ok_or(anyhow!(
            "Could not match account data against any discriminator"
        ))?;
        Ok((
            (type_def.name.clone()),
            self.deserialize_struct_or_enum(type_def, &mut &data[..])?,
        ))
    }

    /// Deserialize an account and output it as a [Value] that is a superset of
    /// [solana_account_decode::UiAccount]
    pub fn try_deserialize_account_to_json(
        &self,
        pubkey: &Pubkey,
        account: &Account,
    ) -> anyhow::Result<DeserializedAccount> {
        let (account_type, deserialized) = self.try_deserialize_account(account)?;
        let ui_account = UiAccount::encode(
            pubkey,
            account,
            UiAccountEncoding::Base64,
            None,
            None,
        );
        Ok(DeserializedAccount {
            ui_account,
            program_name: self.name.clone(),
            account_type,
            deserialized,
        })
    }
}

impl AnchorDeserializer {
    /// Tries to deserialize an account, first trying with any IDL cached from the account's owner,
    /// and failing that, tries to deserialize using all other caches IDLs (order is indeterminate).
    pub fn try_deserialize_account(&self, pubkey: Pubkey, account: &Account) -> Result<DeserializedAccount> {
        if let Some(idl) = self.idl_cache.get(&account.owner) {
            if let Ok(json) = idl.try_deserialize_account_to_json(&pubkey, account) {
                return Ok(json);
            }
        }
        // Brute force search all cached IDLs, trying to deserialize
        for (_, idl) in &self.idl_cache {
            if let Ok(json) = idl.try_deserialize_account_to_json(&pubkey, account) {
                return Ok(json);
            }
        }
        return Err(anyhow!(
            "could not deserialize account from any cached IDLs"
        ));
    }
}