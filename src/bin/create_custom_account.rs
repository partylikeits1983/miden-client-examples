use miden_client::notes::Note;
use miden_objects::accounts::AccountComponent;
use miden_objects::assembly::Assembler;
use rand::Rng;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::fs;
// Import necessary modules and types
use miden_client::{
    accounts::{Account, AccountCode, AccountId, AccountType},
    assets::FungibleAsset,
    config::{Endpoint, RpcConfig},
    crypto::RpoRandomCoin,
    notes::{
        NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient,
        NoteScript, NoteTag, NoteType,
    },
    rpc::TonicRpcClient,
    store::sqlite_store::config::SqliteStoreConfig,
    store::{sqlite_store::SqliteStore, StoreAuthenticator},
    transactions::{
        LocalTransactionProver, PaymentTransactionData, ProvingOptions, TransactionKernel,
        TransactionRequest,
    },
    Client, ClientError, Felt,
};
use miden_lib::accounts::wallets::BasicWallet;

// use miden_crypto::rand::FeltRng;
use miden_lib::StdLibrary;

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

/* fn account_code(assembler: &Assembler) -> AccountCode {
    let account_code = include_str!("../../masm/amm_account.masm");

    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    code
} */

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
    let mut _client = Client::new(
        rpc_client,
        rng_for_client,
        arc_store.clone(),
        Arc::new(authenticator),
        Arc::new(tx_prover),
        false, // set to true if you want debug mode
    );

    let file_path = Path::new("masm/amm_note.masm");

    // Read the file contents
    let masm_code = fs::read_to_string(file_path).expect("Failed to read the file");

    println!("masm: {:?}", masm_code);

    let target: &str = "0x8d03743cf76f4a95";
    let target_account_id = AccountId::from_hex(target).unwrap();
    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;

    let sender = "0x80ed046bc511a83c";
    let sender_account_id = AccountId::from_hex(&sender).unwrap();
    let note_type = NoteType::Private;
    let aux = Felt::new(0);

    let metadata = NoteMetadata::new(
        sender_account_id,
        note_type,
        tag,
        NoteExecutionHint::always(),
        aux,
    )
    .unwrap();

    let assets = [].to_vec();
    let vault = NoteAssets::new(assets)?;

    let serial_num: [Felt; 4] = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let inputs = NoteInputs::new(vec![])?;

    let assembler: Assembler = TransactionKernel::assembler();
    let note_script: NoteScript = NoteScript::compile(masm_code, assembler).unwrap();

    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(vault, metadata, recipient);

    println!("Note hash: {:?}", note.hash());

    // Initializing Account
    let file_path = Path::new("masm/basic_account.masm");

    // Read the file contents
    let masm_code = fs::read_to_string(file_path).expect("Failed to read the file");

    let assembler: Assembler = TransactionKernel::assembler();
    let account_component = AccountComponent::compile(masm_code, assembler, vec![])
        .unwrap()
        .with_supports_all_types();
    let account_code = AccountCode::from_components(
        &[account_component],
        AccountType::RegularAccountUpdatableCode,
    )
    .unwrap();

    println!(
        "number of procedures in account: {:?}",
        account_code.num_procedures()
    );

    Ok(())
}
