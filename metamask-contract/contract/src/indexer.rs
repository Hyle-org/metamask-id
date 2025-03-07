use crate::{actions::IdentityAction, IdentityContractState};
use anyhow::{anyhow, Context, Result};
use client_sdk::contract_indexer::{
    axum::Router,
    utoipa::openapi::OpenApi,
    utoipa_axum::{router::OpenApiRouter, routes},
    ContractHandler, ContractHandlerStore,
};
use client_sdk::contract_indexer::{
    axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    },
    utoipa::{self, ToSchema},
    AppError,
};
use sdk::{tracing::info, Blob, BlobIndex, BlobTransaction, Identity};
use serde::Serialize;

use client_sdk::contract_indexer::axum;
impl ContractHandler for IdentityContractState {
    async fn api(store: ContractHandlerStore<Self>) -> (Router<()>, OpenApi) {
        let (router, api) = OpenApiRouter::default()
            .routes(routes!(get_state))
            .routes(routes!(get_nonce))
            .split_for_parts();

        (router.with_state(store), api)
    }

    fn handle(tx: &BlobTransaction, index: BlobIndex, state: Self) -> Result<Self> {
        let Blob {
            data,
            contract_name,
        } = tx.blobs.get(index.0).context("Failed to get blob")?;

        let action: IdentityAction =
            borsh::from_slice(data.0.as_slice()).context("Failed to decode payload")?;

        let mut blobs = tx.blobs.clone();
        blobs.remove(index.0);

        let identity = tx.identity.clone();

        let res = crate::execute_action(state, action, &"mmid".into(), identity, &blobs)
            .map_err(|e| anyhow::anyhow!(e))?;
        info!("🚀 Executed {contract_name}: {res:?}");
        Ok(res.1)
    }
}

#[utoipa::path(
    get,
    path = "/state",
    tag = "Contract",
    responses(
        (status = OK, description = "Get json state of contract")
    )
)]
pub async fn get_state(
    State(state): State<ContractHandlerStore<IdentityContractState>>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.read().await;
    store.state.clone().map(Json).ok_or(AppError(
        StatusCode::NOT_FOUND,
        anyhow::anyhow!("No state found for contract '{}'", store.contract_name),
    ))
}

#[derive(Serialize, ToSchema)]
struct NonceResponse {
    account: String,
    nonce: u128,
}

#[utoipa::path(
    get,
    path = "/nonce/{account}",
    params(
        ("account" = String, Path, description = "Account")
    ),
    tag = "Contract",
    responses(
        (status = OK, description = "Get nonce of account", body = NonceResponse)
    )
)]
pub async fn get_nonce(
    Path(account): Path<Identity>,
    State(state): State<ContractHandlerStore<IdentityContractState>>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.read().await;
    let state = store.state.clone().ok_or(AppError(
        StatusCode::NOT_FOUND,
        anyhow!("Contract '{}' not found", store.contract_name),
    ))?;

    let info = state
        .get_identity_info(&account.0)
        .map_err(|err| AppError(StatusCode::NOT_FOUND, anyhow::anyhow!(err)))?;

    Ok(Json(NonceResponse {
        account: account.0,
        nonce: info.nonce,
    }))
}
