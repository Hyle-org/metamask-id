use client_sdk::{
    helpers::ClientSdkProver,
    transaction_builder::{StateUpdater, TxExecutorBuilder},
};
use sdk::ContractName;

use crate::IdentityContractState;

pub mod metadata {
    pub const ELF: &[u8] = methods_identity::GUEST_ELF;
}

impl IdentityContractState {
    pub fn setup_builder<S: StateUpdater>(
        &self,
        contract_name: ContractName,
        builder: &mut TxExecutorBuilder<S>,
    ) {
        builder.init_with(contract_name, NoProver {});
    }
}

struct NoProver {}

impl ClientSdkProver for NoProver {
    fn prove(
        &self,
        _contract_input: sdk::ContractInput,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<sdk::ProofData>> + Send + '_>,
    > {
        Box::pin(async move { Err(anyhow::anyhow!("mmm_id proofs will be proved later")) })
    }
}
