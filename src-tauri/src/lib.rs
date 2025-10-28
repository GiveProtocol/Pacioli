mod api;
mod core;
mod db;
mod evm_indexer;
mod indexer;
mod sync;

use evm_indexer::EVMIndexer;
use tokio::sync::Mutex;
use tauri::State;

// Global EVM indexer state
type EVMIndexerState = Mutex<EVMIndexer>;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn connect_evm_chain(
    state: State<'_, EVMIndexerState>,
    chain: String,
) -> Result<String, String> {
    let mut indexer = state.lock().await;
    indexer.connect(&chain).await.map_err(|e| e.to_string())?;
    Ok(format!("Connected to {}", chain))
}

#[tauri::command]
async fn get_evm_balance(
    state: State<'_, EVMIndexerState>,
    chain: String,
    address: String,
) -> Result<String, String> {
    let indexer = state.lock().await;
    let balance = indexer
        .get_balance(&chain, &address)
        .await
        .map_err(|e| e.to_string())?;
    Ok(balance.to_string())
}

#[tauri::command]
async fn get_evm_token_balances(
    state: State<'_, EVMIndexerState>,
    chain: String,
    address: String,
) -> Result<Vec<(String, String)>, String> {
    // Common ERC20 tokens for each chain
    let tokens = match chain.as_str() {
        "moonbeam" => vec![
            "0xAcc15dC74880C9944775448304B263D191c6077F", // WGLMR
            "0x818ec0A7Fe18Ff94269904fCED6AE3DaE6d6dC0b", // USDC
        ],
        "moonriver" => vec![
            "0x98878B06940aE243284CA214f92Bb71a2b032B8A", // WMOVR
            "0xE3F5a90F9cb311505cd691a46596599aA1A0AD7D", // USDC
        ],
        "astar" => vec![
            "0xAeaaf0e2c81Af264101B9129C00F4440cCF0F720", // WASTR
        ],
        _ => vec![],
    };

    let indexer = state.lock().await;
    let balances = indexer
        .scan_erc20_balances(&chain, &address, tokens)
        .await
        .map_err(|e| e.to_string())?;

    Ok(balances
        .into_iter()
        .map(|(addr, balance)| (addr, balance.to_string()))
        .collect())
}

#[tauri::command]
async fn get_evm_transactions(
    state: State<'_, EVMIndexerState>,
    chain: String,
    address: String,
    from_block: u64,
    to_block: String,
) -> Result<Vec<String>, String> {
    let to_block_num = if to_block == "latest" {
        let indexer = state.lock().await;
        indexer
            .get_block_number(&chain)
            .await
            .map_err(|e| e.to_string())?
    } else {
        to_block.parse::<u64>().map_err(|e| e.to_string())?
    };

    let indexer = state.lock().await;
    let transactions = indexer
        .get_transactions(&chain, &address, from_block, to_block_num)
        .await
        .map_err(|e| e.to_string())?;

    // Convert transactions to JSON strings for frontend
    Ok(transactions
        .into_iter()
        .map(|tx| serde_json::to_string(&tx).unwrap_or_default())
        .collect())
}

#[tauri::command]
async fn scan_defi_positions(
    state: State<'_, EVMIndexerState>,
    chain: String,
    address: String,
) -> Result<Vec<String>, String> {
    let protocols = match chain.as_str() {
        "moonbeam" => vec!["stellaswap", "moonwell"],
        "astar" => vec!["arthswap"],
        "acala" => vec!["acala-swap"],
        _ => vec![],
    };

    let indexer = state.lock().await;
    let positions = indexer
        .scan_defi_positions(&chain, &address, protocols)
        .await
        .map_err(|e| e.to_string())?;

    // Convert positions to JSON strings for frontend
    Ok(positions
        .into_iter()
        .map(|pos| serde_json::to_string(&pos).unwrap_or_default())
        .collect())
}

#[tauri::command]
async fn sync_evm_transactions(
    state: State<'_, EVMIndexerState>,
    chain: String,
    address: String,
) -> Result<String, String> {
    // Get latest block and sync from last 1000 blocks
    let indexer = state.lock().await;
    let latest_block = indexer
        .get_block_number(&chain)
        .await
        .map_err(|e| e.to_string())?;
    let from_block = latest_block.saturating_sub(1000);

    let transactions = indexer
        .get_transactions(&chain, &address, from_block, latest_block)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Synced {} transactions", transactions.len()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(EVMIndexerState::new(EVMIndexer::new()))
        .invoke_handler(tauri::generate_handler![
            greet,
            connect_evm_chain,
            get_evm_balance,
            get_evm_token_balances,
            get_evm_transactions,
            scan_defi_positions,
            sync_evm_transactions,
            api::export::export_transactions_csv,
            api::export::export_tax_report,
            api::backup::create_backup,
            api::backup::restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
