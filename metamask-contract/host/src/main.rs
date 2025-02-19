use axum::{extract::Json, http::Method, routing::post, Router};
use clap::{Parser, Subcommand};
use client_sdk::helpers::risc0::Risc0Prover;
use contract_identity::IdentityContractState;
use hex::decode;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use sdk::api::APIRegisterContract;
use sdk::TxHash;
use sdk::{ContractInput, Digestable};
use sdk::{Identity, ProofTransaction};
use serde::Deserialize;
use sha3::Digest;
use sha3::Keccak256;
use std::env;
use tower_http::cors::{Any, CorsLayer};

// These constants represent the RISC-V ELF and the image ID generated by risc0-build.
// The ELF is used for proving and the ID is used for verification.
use methods_identity::{GUEST_ELF, GUEST_ID};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[clap(long, short)]
    reproducible: bool,

    #[arg(long, default_value = "http://localhost:4321")]
    pub host: String,

    #[arg(long, default_value = "mmid")]
    pub contract_name: String,
}

#[derive(Subcommand)]
enum Commands {
    RunServer,
    RegisterContract {},
    ValidateSignature { account: String, signature: String },
}

#[derive(Deserialize)]
struct ProveRequest {
    tx_hash: TxHash,
    contract_name: String,
    identity: Identity,
    signature: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let client = client_sdk::rest_client::NodeApiHttpClient::new(cli.host).unwrap();

    let contract_name = &cli.contract_name;

    match cli.command {
        Commands::RegisterContract {} => {
            // Build initial state of contract
            let initial_state = IdentityContractState::new();
            println!("Initial state: {:?}", initial_state);

            // Send the transaction to register the contract
            let register_tx = APIRegisterContract {
                verifier: "risc0".into(),
                program_id: sdk::ProgramId(sdk::to_u8_array(&GUEST_ID).to_vec()),
                state_digest: initial_state.as_digest(),
                contract_name: contract_name.clone().into(),
            };
            let res = client.register_contract(&register_tx).await.unwrap();

            println!("✅ Register contract tx sent. Tx hash: {}", res);
        }
        Commands::ValidateSignature { signature, account } => {
            //Example \`personal_sign\` message
            //0xc4b1989d045e1f9aacc448032a7e278780de9a1c1735984c8d4e95cc1840715b3255b0cb791df3c5c137fa22773f9f6976b96418581e44f3fdf1e6ec395f6b661b
            //0x437aa724e898f0ba345852bbbc2e416d9346e1c9

            //hyle registration
            //0x3279f925d976ddfc012a95157f87054717610b4fa08028913ab3139f552e76342d609c11acbca164fb9c64fb5553db3fe34c826c0084fd6bc934dcd154993b0a1b
            //0x437aa724e898f0ba345852bbbc2e416d9346e1c9

            let res = k256_verifier(account, signature);

            if res {
                println!("✅ Signature successfully validated.");
            } else {
                println!("❌ Signature invalid");
            }
        }

        Commands::RunServer => {
            run_server().await;
        }
    }
}

// Function to start the REST server
async fn run_server() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::POST])
        .allow_headers(Any);

    let app = Router::new().route("/prove", post(prove)).layer(cors);

    let addr = env::var("HYLEOOF_HOST").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on {}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// Handler for /prove endpoint
async fn prove(Json(request): Json<ProveRequest>) -> Json<TxHash> {
    let cli = Cli::parse();
    let client = client_sdk::rest_client::NodeApiHttpClient::new(cli.host.clone()).unwrap();
    let indexer = client_sdk::rest_client::IndexerApiHttpClient::new(cli.host).unwrap();
    let prover = Risc0Prover::new(GUEST_ELF);

    let initial_state: IdentityContractState = client
        .get_contract(&request.contract_name.clone().into())
        .await
        .unwrap()
        .state
        .try_into()
        .unwrap();

    let blobs: Vec<sdk::Blob> = indexer
        .get_blobs_by_tx_hash(&request.tx_hash)
        .await
        .unwrap()
        .into_iter()
        .map(|blob| sdk::Blob {
            contract_name: blob.contract_name.clone().into(),
            data: sdk::BlobData(blob.data),
        })
        .collect();

    println!("Initial state {:?}", initial_state.clone());
    println!("identity {:?}", request.identity.clone());
    println!("signature {:?}", request.signature.clone());
    println!("contract_name {:?}", request.contract_name.clone());
    println!("tx_hash {:?}", request.tx_hash.clone());

    let inputs = ContractInput {
        initial_state: initial_state.as_digest(),
        identity: request.identity.clone().into(),
        tx_hash: request.tx_hash.clone().into(),
        private_input: vec![],
        blobs: blobs.clone(),
        index: sdk::BlobIndex(0),
        tx_ctx: None,
    };

    println!("inputs {:?}", inputs.clone());

    let res = contract_identity::execute(inputs.clone());
    if let Err(e) = res {
        println!("Error: {:?}", e);
    }

    let proof = prover.prove(inputs).await.unwrap();
    let proof_tx = ProofTransaction {
        proof,
        contract_name: request.contract_name.clone().into(),
    };

    let proof_tx_hash: TxHash = client.send_tx_proof(&proof_tx).await.unwrap();
    //println!("Proof transaction sent: {:?}", proof.clone());
    Json(proof_tx_hash)
}

pub fn k256_verifier(pub_key: String, signature_hex: String) -> bool {
    let sig = &String::from(signature_hex);
    let pb = &String::from(pub_key);

    let signature_hex_str = sanitize_hex(sig);
    let pub_key_str = sanitize_hex(pb);

    let msg = "hyle registration".as_bytes();

    // Apply Ethereum Signed Message Prefix (EIP-191)
    let eth_message = format!(
        "\x19Ethereum Signed Message:\n{}{}",
        msg.len(),
        String::from_utf8_lossy(msg)
    );

    let signature_bytes = decode(&signature_hex_str).expect("Invalid hex string");
    let recovery_id_byte = signature_bytes.last().copied().expect("Signature is empty");
    // Normalize Ethereum's recovery ID
    let recovery_id_byte = if recovery_id_byte >= 27 {
        recovery_id_byte - 27
    } else {
        recovery_id_byte
    };
    let recovery_id = RecoveryId::try_from(recovery_id_byte).expect("Wrong recover id byte");

    let signature = Signature::from_slice(&signature_bytes[..64]).expect("wrong signature");

    let recovered_key = VerifyingKey::recover_from_digest(
        Keccak256::new_with_prefix(eth_message),
        &signature,
        recovery_id,
    )
    .expect("wrong recovered key");

    let encoded_point = recovered_key.to_encoded_point(false);
    let pub_key_bytes = encoded_point.as_bytes();

    // 6️⃣ Hash the public key (skip the first byte which is always 0x04)
    let hashed_key = Keccak256::digest(&pub_key_bytes[1..]);

    // 7️⃣ Extract the last 20 bytes (Ethereum address format)
    let recovered_address = &hashed_key[12..]; // Last 20 bytes
    hex::encode(recovered_address) == pub_key_str
}

fn sanitize_hex(hex_str: &str) -> &str {
    hex_str.strip_prefix("0x").unwrap_or(hex_str)
}
