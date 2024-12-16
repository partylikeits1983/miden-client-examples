use rand::Rng;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Import necessary modules and types
use miden_client::{
    accounts::AccountId,
    assets::FungibleAsset,
    config::{Endpoint, RpcConfig},
    crypto::RpoRandomCoin,
    notes::NoteType,
    rpc::TonicRpcClient,
    store::sqlite_store::config::SqliteStoreConfig,
    store::{sqlite_store::SqliteStore, StoreAuthenticator},
    transactions::{
        LocalTransactionProver, PaymentTransactionData, ProvingOptions, TransactionRequest,
    },
    Client, ClientError, Felt,
};

use figment::{
    providers::{Format, Toml},
    Figment,
};

const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    /// Describes settings related to the RPC endpoint
    pub rpc: RpcConfig,
    /// Describes settings related to the store.
    pub store: SqliteStoreConfig,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
    /// Path to the file containing the token symbol map.
    pub token_symbol_map_filepath: PathBuf,
    /// RPC endpoint for the proving service. If this is not present, a local prover will be used.
    pub remote_prover_endpoint: Option<Endpoint>,
}

impl ClientConfig {
    fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let figment = Figment::from(Toml::file(path));
        figment.extract().unwrap_or_else(|e| {
            panic!("Failed to load client config: {}", e);
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // Load configuration from file
    let client_config = ClientConfig::from_file(CLIENT_CONFIG_FILE_NAME);

    // Initialize the store
    let store = SqliteStore::new(&client_config.store)
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    // Seed an RNG for cryptographic randomness
    let mut seed_rng = rand::thread_rng();
    let coin_seed: [u64; 4] = seed_rng.gen();

    // Create two RNGs since `new_with_rng` consumes one
    let rng_for_auth = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let rng_for_client = RpoRandomCoin::new(coin_seed.map(Felt::new));

    // Create authenticator
    let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng_for_auth);

    // Create a local transaction prover
    let tx_prover = LocalTransactionProver::new(ProvingOptions::default());

    // Create the RPC client trait object
    let rpc_client = Box::new(TonicRpcClient::new(&client_config.rpc));

    // Instantiate the client
    let mut client = Client::new(
        rpc_client,
        rng_for_client,
        arc_store.clone(),
        Arc::new(authenticator),
        Arc::new(tx_prover),
        false, // set to true if you want debug mode
    );

    let faucet_id = "0x2a021ef7df708360"; // id ouputted by miden new-faucet
    let bob_account_id = "0x80ed046bc511a83c"; // id outputted by miden new-wallet
    let alice_account_id = "0x8d03743cf76f4a95"; // id outputted by miden new-wallet
    let amount: u64 = 3; // Example amount

    let faucet_id = AccountId::from_hex(faucet_id)?;
    let sender_account_id = AccountId::from_hex(bob_account_id)?;
    let target_account_id = AccountId::from_hex(alice_account_id)?;

    let fungible_asset = FungibleAsset::new(faucet_id, amount).unwrap().into();
    let payment_transaction =
        PaymentTransactionData::new(vec![fungible_asset], sender_account_id, target_account_id);

    let transaction_request =
        TransactionRequest::pay_to_id(payment_transaction, None, NoteType::Private, client.rng())?;

    // Execute the transaction
    let transaction_execution_result = client
        .new_transaction(sender_account_id, transaction_request.clone())
        .await?;
    println!(
        "Transaction executed locally: {:?}",
        transaction_execution_result.block_num()
    );

    println!("submitting transaction...");

    // Prove and submit the transaction
    client
        .submit_transaction(transaction_execution_result)
        .await?;
    println!("Transaction submitted successfully.");

    Ok(())
}
