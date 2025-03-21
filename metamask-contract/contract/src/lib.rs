use actions::IdentityAction;
use borsh::{BorshDeserialize, BorshSerialize};
use hex::{decode, encode};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use sdk::{utils::parse_raw_contract_input, HyleContract, RunResult};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha3::Keccak256;
use std::collections::BTreeMap;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub mod indexer;

pub mod actions;

extern crate alloc;

impl HyleContract for IdentityContractState {
    /// Entry point of the contract's logic
    fn execute(&mut self, input: &sdk::ContractInput) -> RunResult {
        // Parse contract inputs
        let (action, ctx) = parse_raw_contract_input::<IdentityAction>(input)?;

        let identity = input.identity.clone();
        let contract_name = &input
            .blobs
            .get(input.index.0)
            .ok_or("No blob")?
            .contract_name;

        let program_output = if input.index.0 == 0 {
            // Identity blob should be at position 0
            let blobs = input
                .blobs
                .split_first()
                .map(|(_, rest)| rest)
                .ok_or("No blobs")?;
            self.execute_action(action, contract_name, identity, blobs)?
        } else {
            // Otherwise, it's less efficient as need to clone blobs & the remove is O(n)
            let mut blobs = input.blobs.clone();
            blobs.remove(input.index.0);
            self.execute_action(action, contract_name, identity, &blobs)?
        };

        Ok((program_output, ctx, vec![]))
    }

    fn commit(&self) -> sdk::StateCommitment {
        sdk::StateCommitment(self.as_bytes().expect("Failed to encode state"))
    }
}

/// Struct to hold account's information
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AccountInfo {
    pub pub_key_hash: String,
    pub nonce: u128,
}

/// The state of the contract, that is totally serialized on-chain
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct IdentityContractState {
    identities: BTreeMap<String, AccountInfo>,
}

/// Some helper methods for the state
impl IdentityContractState {
    pub fn new() -> Self {
        IdentityContractState {
            identities: BTreeMap::new(),
        }
    }

    pub fn get_nonce(&self, account: &str) -> Result<u128, &'static str> {
        let info = self.identities.get(account).ok_or("Identity not found")?;
        Ok(info.nonce)
    }
}

impl IdentityContractState {
    pub fn execute_action(
        &mut self,
        action: IdentityAction,
        contract_name: &sdk::ContractName,
        account: sdk::Identity,
        blobs: &[sdk::Blob],
    ) -> Result<String, String> {
        if !account.0.ends_with(&contract_name.0) {
            return Err(format!(
                "Invalid account extension. '.{contract_name}' expected."
            ));
        }
        let pub_key = account
            .0
            .trim_end_matches(&contract_name.0)
            .trim_end_matches(".");

        match action {
            IdentityAction::RegisterIdentity { signature } => {
                self.register_identity(pub_key, &signature)
            }
            IdentityAction::VerifyIdentity { nonce, signature } => {
                match self.verify_identity(pub_key, nonce, blobs, &signature) {
                    Ok(true) => Ok(format!("Identity verified for account: {}", account)),
                    Ok(false) => Err(format!(
                        "Identity verification failed for account: {}",
                        account
                    )),
                    Err(err) => Err(format!("Error verifying identity: {}", err)),
                }
            }
        }
    }

    fn register_identity(&mut self, pub_key: &str, signature: &str) -> Result<String, String> {
        // Parse the signature
        let valid = k256_verifier(pub_key, signature, "hyle registration");

        if !valid {
            return Err(format!(
                "Invalid register signature for {pub_key}, {signature}"
            ));
        }

        let pub_key_hash = Keccak256::digest(pub_key.as_bytes());
        let pub_key_hash_hex = encode(pub_key_hash);

        let account_info = AccountInfo {
            pub_key_hash: pub_key_hash_hex,
            nonce: 0,
        };

        if self
            .identities
            .insert(pub_key.to_string(), account_info)
            .is_some()
        {
            return Err("Identity already exists".to_string());
        }

        Ok("Identity registered".to_string())
    }

    fn verify_identity(
        &mut self,
        pub_key: &str,
        nonce: u128,
        blobs: &[sdk::Blob],
        signature: &str,
    ) -> Result<bool, String> {
        match self.identities.get_mut(pub_key) {
            Some(stored_info) => {
                if nonce < stored_info.nonce {
                    return Err("Invalid nonce".to_string());
                }
                // Compute Keccak256 hash of the account (to match register_identity)
                let pub_key_hash = Keccak256::digest(pub_key.as_bytes());
                let computed_hash = encode(pub_key_hash);

                if *stored_info.pub_key_hash != computed_hash {
                    return Ok(false);
                }

                // const message = `verify ${nonce} ${blobs.map((blob) => blob.contract_name + " " + blob.data).join(" ")}`;

                let message = blobs
                    .iter()
                    .map(|blob| format!("{} {:?}", blob.contract_name, blob.data.0))
                    .collect::<Vec<String>>()
                    .join(" ");
                let message = format!("verify {} {}", nonce, message);

                let valid = k256_verifier(pub_key, signature, &message);

                if !valid {
                    return Err(format!("Invalid signature for message {message}"));
                }

                stored_info.nonce = nonce + 1;
                Ok(true)
            }
            None => Err("Identity not found".to_string()),
        }
    }

    #[allow(dead_code)]
    pub fn get_identity_info(&self, account: &str) -> Result<AccountInfo, &'static str> {
        self.identities
            .get(account)
            .cloned()
            .ok_or("Identity not found")
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, borsh::io::Error> {
        borsh::to_vec(self)
    }
}

impl Default for IdentityContractState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helpers to transform the contrat's state commitment in its full-state.
/// In an optimal version, you would here only returns a hash of the state,
/// while storing the full-state off-chain
impl TryFrom<sdk::StateCommitment> for IdentityContractState {
    type Error = anyhow::Error;

    fn try_from(state: sdk::StateCommitment) -> Result<Self, Self::Error> {
        borsh::from_slice(&state.0)
            .map_err(|_| anyhow::anyhow!("Could not decode identity state".to_string()))
    }
}

pub fn k256_verifier(mut pub_key: &str, mut signature_hex: &str, message: &str) -> bool {
    pub_key = sanitize_hex(pub_key);
    signature_hex = sanitize_hex(signature_hex);

    let msg = message.as_bytes();

    // Apply Ethereum Signed Message Prefix (EIP-191)
    let eth_message = format!(
        "\x19Ethereum Signed Message:\n{}{}",
        msg.len(),
        String::from_utf8_lossy(msg)
    );

    let signature_bytes = decode(signature_hex).expect("Invalid hex string");
    let recovery_id_byte = signature_bytes.last().copied().expect("Signature is empty");
    // Normalize Ethereum's recovery ID
    let recovery_id_byte = if recovery_id_byte >= 27 {
        recovery_id_byte - 27
    } else {
        recovery_id_byte
    };
    let recovery_id = RecoveryId::try_from(recovery_id_byte).expect("Wrong recover id byte");

    let signature = Signature::from_slice(&signature_bytes[..64]).expect("Wrong signature");

    let recovered_key = VerifyingKey::recover_from_digest(
        Keccak256::new_with_prefix(eth_message),
        &signature,
        recovery_id,
    )
    .expect("Error when recovering public key");

    let encoded_point = recovered_key.to_encoded_point(false);
    let pub_key_bytes = encoded_point.as_bytes();

    // Hash the public key (skip the first byte which is always 0x04)
    let hashed_key = Keccak256::digest(&pub_key_bytes[1..]);

    // Extract the last 20 bytes (Ethereum address format)
    let recovered_address = &hashed_key[12..]; // Last 20 bytes
    hex::encode(recovered_address) == pub_key
}

fn sanitize_hex(hex_str: &str) -> &str {
    hex_str.strip_prefix("0x").unwrap_or(hex_str)
}
