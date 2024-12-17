use rand::Rng;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use miden_client::{
    accounts::{AccountId, AccountStorageMode, AccountTemplate},
    assets::{FungibleAsset, TokenSymbol},
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

use tokio::time::Duration;

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
    //------------------------------------------------------------
    // Load client configuration
    //------------------------------------------------------------
    let client_config = ClientConfig::from_file(CLIENT_CONFIG_FILE_NAME);

    // Initialize the store
    let store = SqliteStore::new(&client_config.store)
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    // Seed RNG for cryptographic randomness
    let mut seed_rng = rand::thread_rng();
    let coin_seed: [u64; 4] = seed_rng.gen();
    let rng_for_auth = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let rng_for_client = RpoRandomCoin::new(coin_seed.map(Felt::new));

    // Create authenticator
    let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng_for_auth);

    // Create local transaction prover
    let tx_prover = LocalTransactionProver::new(ProvingOptions::default());

    // Create the RPC client
    let rpc_client = Box::new(TonicRpcClient::new(&client_config.rpc));

    // Instantiate the client
    let mut client = Client::new(
        rpc_client,
        rng_for_client,
        arc_store.clone(),
        Arc::new(authenticator),
        Arc::new(tx_prover),
        false,
    );

    //------------------------------------------------------------
    // STEP 1: Create a basic wallet account for Alice
    //------------------------------------------------------------
    println!("\n[STEP 1] Creating new account for Alice");

    let alice_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: AccountStorageMode::Private,
    };

    // Create Alice's account
    let (alice_account, _alice_seed) = client.new_account(alice_template).await?;
    println!(
        "Successfully created Alice's wallet. ID: {:?}",
        alice_account.id()
    );

    //------------------------------------------------------------
    // STEP 2: Deploy a fungible faucet (token)
    //------------------------------------------------------------
    println!("\n[STEP 2] Deploying a new fungible faucet.");

    // Token configuration
    let token_symbol_str = "POL";
    let decimals = 8;
    let max_supply = 1_000_000;

    let faucet_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new(token_symbol_str).expect("Token symbol is invalid"),
        decimals,
        max_supply,
        storage_mode: AccountStorageMode::Public,
    };

    let (faucet_account, _faucet_seed) = client.new_account(faucet_template).await?;
    println!(
        "Successfully created a new faucet. Faucet ID: {:?}",
        faucet_account.id()
    );

    //------------------------------------------------------------
    // STEP 3: Mint 5 notes of 100 tokens each for Alice
    //------------------------------------------------------------
    println!("\n[STEP 3] Minting 5 notes of 100 tokens each for Alice.");

    client.sync_state().await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    for i in 1..=5 {
        let amount = 100;
        let fungible_asset = FungibleAsset::new(faucet_account.id(), amount)
            .expect("Failed to create fungible asset struct.");

        let transaction_request = TransactionRequest::mint_fungible_asset(
            fungible_asset.clone(),
            alice_account.id(),
            NoteType::Private,
            client.rng(),
        )
        .expect("Failed to create mint transaction request.");

        let tx_execution_result = client
            .new_transaction(faucet_account.id(), transaction_request)
            .await?;

        client.submit_transaction(tx_execution_result).await?;
        println!("Minted note #{} of {} tokens for Alice.", i, amount);
    }

    // Sync state to ensure all notes are visible to the client
    client.sync_state().await?;
    println!("All 5 notes minted for Alice successfully!");

    //------------------------------------------------------------
    // STEP 4: Alice consumes all her notes
    //------------------------------------------------------------
    println!("\n[STEP 4] Alice will now consume all of her notes to consolidate them.");

    // Wait until there are exactly 5 consumable notes for Alice
    loop {
        // Re-sync state to ensure we have the latest info
        client.sync_state().await?;

        // Fetch all consumable notes for Alice
        let consumable_notes = client
            .get_consumable_notes(Some(alice_account.id()))
            .await?;
        let list_of_note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();

        if list_of_note_ids.len() == 5 {
            println!(
                "Alice has {} consumable notes. Consuming them now...",
                list_of_note_ids.len()
            );

            let transaction_request = TransactionRequest::consume_notes(list_of_note_ids);
            let tx_execution_result = client
                .new_transaction(alice_account.id(), transaction_request)
                .await?;

            client.submit_transaction(tx_execution_result).await?;
            println!("Successfully consumed all of Alice's notes.");
            break;
        } else {
            println!(
                "Currently, Alice has {} consumable notes. Waiting for 5...",
                list_of_note_ids.len()
            );
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    //------------------------------------------------------------
    // STEP 5: Using Alice's wallet, send 5 notes of 50 tokens each to list of users
    //------------------------------------------------------------
    println!("\n[STEP 5] Alice sends 5 notes of 50 tokens each to 5 different users.");

    for i in 1..=5 {
        let target_account_string = format!("0x{:016x}", i);
        let target_account_id = AccountId::from_hex(&target_account_string).unwrap();

        let send_amount = 50;
        let fungible_asset = FungibleAsset::new(faucet_account.id(), send_amount)
            .expect("Failed to create fungible asset for sending.");

        let payment_transaction = PaymentTransactionData::new(
            vec![fungible_asset.into()],
            alice_account.id(),
            target_account_id,
        );

        // Create a pay-to-id transaction
        let transaction_request = TransactionRequest::pay_to_id(
            payment_transaction,
            None,              // recall_height: None
            NoteType::Private, // note type is private
            client.rng(),      // rng
        )
        .expect("Failed to create payment transaction request.");

        let tx_execution_result = client
            .new_transaction(alice_account.id(), transaction_request)
            .await?;

        client.submit_transaction(tx_execution_result).await?;
        println!(
            "Sent note #{} of {} tokens to AccountId {}.",
            i, send_amount, target_account_id
        );
    }

    println!("\nAll steps completed successfully!");
    println!("Alice created a wallet, Bob created a wallet, a faucet was deployed,");
    println!("5 notes of 100 tokens were minted to Alice, those notes were consumed,");
    println!("and then Alice sent 5 separate 50-token notes to 5 different users.");

    Ok(())
}
