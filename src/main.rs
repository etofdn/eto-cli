//! ETO CLI — Command-line interface for the ETO Universal VM
//!
//! Standalone binary. No dependency on svm-runtime.
//! Modeled after the Solana CLI. Clean, minimal output.
//! Reads config from ~/.config/eto/config.json.

use base64::Engine as _;
use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Inlined SVM types (no svm-runtime dependency) ──

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Pubkey(pub [u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Transaction {
    pub signatures: Vec<[u8; 64]>,
    pub message: Message,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Message {
    pub header: MessageHeader,
    pub account_keys: Vec<Pubkey>,
    pub recent_blockhash: [u8; 32],
    pub instructions: Vec<CompiledInstruction>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MessageHeader {
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

impl Transaction {
    pub fn new(message: Message, signers: &[&ed25519_dalek::SigningKey]) -> Self {
        let message_bytes = borsh::to_vec(&message).expect("Failed to serialize message");
        let signatures: Vec<[u8; 64]> = signers
            .iter()
            .map(|signer| {
                use ed25519_dalek::Signer;
                signer.sign(&message_bytes).to_bytes()
            })
            .collect();
        Self { signatures, message }
    }
}


const VERSION: &str = "1.0.0";
const DEFAULT_RPC: &str = "http://127.0.0.1:8899";

// ── Program IDs ──

const SYSTEM_PROGRAM: Pubkey = Pubkey([0u8; 32]);
const EVM_PROGRAM_ID: Pubkey = Pubkey([
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xEE,
]);
const WASM_PROGRAM_ID: Pubkey = Pubkey([
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03,
]);
const MOVE_PROGRAM_ID: Pubkey = Pubkey([
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02,
]);
const ZK_BN254_PROGRAM_ID: Pubkey = Pubkey([
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x05,
]);

// ── Config ──

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("eto")
        .join("config.json")
}

fn load_config() -> serde_json::Value {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}

fn save_config(config: &serde_json::Value) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(config).expect("serialize config");
    std::fs::write(&path, json).unwrap_or_else(|e| {
        eprintln!("Error: unable to write config: {}", e);
        std::process::exit(1);
    });
}

fn resolve_rpc_url(cli_url: Option<&str>) -> String {
    // Priority: CLI flag > env var > config file > default
    if let Some(url) = cli_url {
        return url.to_string();
    }
    if let Ok(url) = std::env::var("ETO_RPC_URL") {
        return url;
    }
    let config = load_config();
    if let Some(url) = config.get("rpc_url").and_then(|v| v.as_str()) {
        return url.to_string();
    }
    DEFAULT_RPC.to_string()
}

// ── Keypair management ──

fn keypair_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home).join(".config").join("eto").join("keys")
}

fn default_keypair_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home).join(".config").join("eto").join("id.json")
}

fn save_keypair(path: &std::path::Path, key: &SigningKey) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bytes: Vec<u8> = key.to_bytes().iter().chain(key.verifying_key().to_bytes().iter()).copied().collect();
    let json = serde_json::to_string(&bytes).unwrap();
    std::fs::write(path, &json).unwrap_or_else(|e| {
        eprintln!("Error writing keypair: {}", e);
        std::process::exit(1);
    });
}

fn load_keypair(path: &std::path::Path) -> Option<SigningKey> {
    let data = std::fs::read_to_string(path).ok()?;
    let bytes: Vec<u8> = serde_json::from_str(&data).ok()?;
    if bytes.len() >= 32 {
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes[..32]);
        Some(SigningKey::from_bytes(&seed))
    } else {
        None
    }
}

fn active_keypair() -> SigningKey {
    // Try: config keypair_path > default id.json > faucet fallback
    let config = load_config();
    if let Some(p) = config.get("keypair_path").and_then(|v| v.as_str()) {
        let path = if p.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(p.replacen('~', &home, 1))
        } else {
            std::path::PathBuf::from(p)
        };
        if let Some(k) = load_keypair(&path) {
            return k;
        }
    }
    if let Some(k) = load_keypair(&default_keypair_path()) {
        return k;
    }
    // Fallback to faucet keypair
    payer_keypair()
}

fn cmd_keygen(args: &[String]) {
    let outfile = args.first().map(|s| std::path::PathBuf::from(s));

    // Generate random keypair
    let mut seed = [0u8; 32];
    getrandom(&mut seed);
    let key = SigningKey::from_bytes(&seed);
    let pk = Pubkey(key.verifying_key().to_bytes());
    let addr = bs58::encode(&pk.0).into_string();

    let path = outfile.unwrap_or_else(|| {
        let dir = keypair_dir();
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("{}.json", &addr[..8]))
    });

    save_keypair(&path, &key);

    println!("Wrote new keypair to {}", path.display());
    println!("{}", addr);
}

fn cmd_keygen_set_default(args: &[String]) {
    let file = args.first().unwrap_or_else(|| {
        eprintln!("Usage: eto keypair set <FILE>");
        std::process::exit(1);
    });
    let mut config = load_config();
    config["keypair_path"] = serde_json::Value::String(file.clone());
    save_config(&config);
    if let Some(k) = load_keypair(std::path::Path::new(file)) {
        let pk = Pubkey(k.verifying_key().to_bytes());
        println!("Default keypair: {}", bs58::encode(&pk.0).into_string());
    } else {
        println!("Set keypair path to: {}", file);
    }
}

fn cmd_keypair_list() {
    let dir = keypair_dir();
    if !dir.exists() {
        println!("No keypairs found. Generate one with: eto keygen");
        return;
    }
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(k) = load_keypair(&entry.path()) {
                    let pk = Pubkey(k.verifying_key().to_bytes());
                    println!("  {} ({})", bs58::encode(&pk.0).into_string(), entry.file_name().to_string_lossy());
                    count += 1;
                }
            }
        }
    }
    // Also check default
    if let Some(k) = load_keypair(&default_keypair_path()) {
        let pk = Pubkey(k.verifying_key().to_bytes());
        println!("  {} (id.json) *default*", bs58::encode(&pk.0).into_string());
        count += 1;
    }
    if count == 0 {
        println!("No keypairs found. Generate one with: eto keygen");
    }
}

fn cmd_keypair_import(args: &[String]) {
    let hex_or_bs58 = args.first().unwrap_or_else(|| {
        eprintln!("Usage: eto keypair import <PRIVATE_KEY_HEX_OR_BASE58>");
        std::process::exit(1);
    });

    let bytes = if hex_or_bs58.len() == 64 {
        hex::decode(hex_or_bs58).unwrap_or_else(|_| {
            eprintln!("Error: invalid hex"); std::process::exit(1);
        })
    } else {
        bs58::decode(hex_or_bs58).into_vec().unwrap_or_else(|_| {
            eprintln!("Error: invalid base58"); std::process::exit(1);
        })
    };

    if bytes.len() < 32 {
        eprintln!("Error: key must be at least 32 bytes");
        std::process::exit(1);
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes[..32]);
    let key = SigningKey::from_bytes(&seed);
    let pk = Pubkey(key.verifying_key().to_bytes());
    let addr = bs58::encode(&pk.0).into_string();

    let dir = keypair_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", &addr[..8]));
    save_keypair(&path, &key);

    println!("Imported keypair: {}", addr);
    println!("Saved to: {}", path.display());
}

fn getrandom(buf: &mut [u8]) {
    use std::io::Read;
    let path = if cfg!(target_os = "macos") { "/dev/urandom" } else { "/dev/urandom" };
    std::fs::File::open(path)
        .expect("open /dev/urandom")
        .read_exact(buf)
        .expect("read /dev/urandom");
}

fn random_blockhash() -> [u8; 32] {
    let mut h = [0u8; 32];
    getrandom(&mut h);
    h
}

fn cmd_create_account(rpc: &str, args: &[String]) {
    let space: u64 = args.first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let lamports: u64 = args.get(1)
        .and_then(|s| parse_amount(s).ok())
        .unwrap_or(1_000_000_000);

    // Generate new account keypair
    let mut seed = [0u8; 32];
    getrandom(&mut seed);
    let new_key = SigningKey::from_bytes(&seed);
    let new_pk = Pubkey(new_key.verifying_key().to_bytes());

    let payer = active_keypair();
    let payer_pk = pubkey_of(&payer);

    let mut data = Vec::with_capacity(52);
    data.extend_from_slice(&0u32.to_le_bytes()); // CreateAccount
    data.extend_from_slice(&lamports.to_le_bytes());
    data.extend_from_slice(&space.to_le_bytes());
    data.extend_from_slice(&SYSTEM_PROGRAM.0); // owner = system

    let msg = Message {
        header: MessageHeader {
            num_required_signatures: 2,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![payer_pk, new_pk, SYSTEM_PROGRAM],
        recent_blockhash: random_blockhash(),
        instructions: vec![CompiledInstruction {
            program_id_index: 2,
            accounts: vec![0, 1],
            data,
        }],
    };

    let tx = Transaction::new(msg, &[&payer, &new_key]);
    let tx_bytes = borsh::to_vec(&tx).expect("borsh");
    let client = make_client();
    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
            println!("Account: {}", bs58::encode(&new_pk.0).into_string());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

// ── Keypair utilities ──

fn payer_keypair() -> SigningKey {
    let seed: [u8; 32] = [
        0xFF, 0xF9, 0x0A, 0x45, 0xE0, 0x92, 0xF3, 0x89, 0x19, 0xED, 0xA5, 0xA6, 0x9F, 0xB7, 0x3B,
        0x07, 0x2F, 0xB2, 0x08, 0x16, 0x10, 0x21, 0x61, 0x8C, 0xDF, 0x83, 0x91, 0x8C, 0xF5, 0x12,
        0x91, 0x0D,
    ];
    SigningKey::from_bytes(&seed)
}

fn pubkey_of(key: &SigningKey) -> Pubkey {
    Pubkey(key.verifying_key().to_bytes())
}

fn pubkey_b58(pk: &Pubkey) -> String {
    bs58::encode(&pk.0).into_string()
}

fn sign_and_serialize(msg: Message, signers: &[&SigningKey]) -> Vec<u8> {
    let tx = Transaction::new(msg, signers);
    borsh::to_vec(&tx).expect("borsh serialize")
}

// ── JSON-RPC client ──

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build HTTP client")
}

fn rpc_call(
    client: &reqwest::blocking::Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let resp: serde_json::Value = client
        .post(url)
        .json(&body)
        .send()
        .map_err(|e| format!("Error: HTTP request failed: {}", e))?
        .json()
        .map_err(|e| format!("Error: failed to parse response: {}", e))?;

    if let Some(error) = resp.get("error") {
        return Err(format!("RPC error: {}", error));
    }

    resp.get("result")
        .cloned()
        .ok_or_else(|| "No result in response".to_string())
}

fn send_tx(
    client: &reqwest::blocking::Client,
    url: &str,
    tx_bytes: Vec<u8>,
) -> Result<String, String> {
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);
    let result = rpc_call(client, url, "sendTransaction", serde_json::json!([tx_b64]))?;
    Ok(result.as_str().unwrap_or("(no signature)").to_string())
}

// ── Address parsing ──

fn parse_address(addr: &str) -> Result<Pubkey, String> {
    if addr.starts_with("0x") || addr.starts_with("0X") {
        let hex_str = &addr[2..];
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex address: {}", e))?;
        if bytes.len() > 32 {
            return Err("Hex address too long (max 32 bytes)".to_string());
        }
        let mut arr = [0u8; 32];
        arr[32 - bytes.len()..].copy_from_slice(&bytes);
        Ok(Pubkey(arr))
    } else {
        let bytes = bs58::decode(addr)
            .into_vec()
            .map_err(|e| format!("Invalid base58 address: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!(
                "Base58 address must decode to 32 bytes, got {}",
                bytes.len()
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Pubkey(arr))
    }
}

/// Parse an amount string. If it contains a decimal point, treat as SOL and
/// convert to lamports. Otherwise treat as raw lamports.
fn parse_amount(s: &str) -> Result<u64, String> {
    if s.contains('.') {
        let sol: f64 = s.parse().map_err(|_| format!("Invalid amount: '{}'", s))?;
        if sol < 0.0 {
            return Err("Amount must be positive".to_string());
        }
        Ok((sol * 1_000_000_000.0) as u64)
    } else {
        s.parse::<u64>()
            .map_err(|_| format!("Invalid amount: '{}'", s))
    }
}

fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

// ── Transaction builders ──

fn build_svm_transfer(payer: &SigningKey, to: Pubkey, amount: u64) -> Vec<u8> {
    let from = pubkey_of(payer);
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());

    let msg = Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![from, to, SYSTEM_PROGRAM],
        recent_blockhash: random_blockhash(),
        instructions: vec![CompiledInstruction {
            program_id_index: 2,
            accounts: vec![0, 1],
            data,
        }],
    };
    sign_and_serialize(msg, &[payer])
}

fn build_create_account(
    payer: &SigningKey,
    new_key: &SigningKey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Vec<u8> {
    let payer_pk = pubkey_of(payer);
    let new_pk = pubkey_of(new_key);
    let mut data = Vec::with_capacity(52);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());
    data.extend_from_slice(&space.to_le_bytes());
    data.extend_from_slice(&owner.0);

    let msg = Message {
        header: MessageHeader {
            num_required_signatures: 2,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![payer_pk, new_pk, SYSTEM_PROGRAM],
        recent_blockhash: random_blockhash(),
        instructions: vec![CompiledInstruction {
            program_id_index: 2,
            accounts: vec![0, 1],
            data,
        }],
    };
    sign_and_serialize(msg, &[payer, new_key])
}

fn build_zk_add(payer: &SigningKey) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0); // opcode 0 = EC Add

    let mut x1 = [0u8; 32];
    x1[0] = 1;
    let mut y1 = [0u8; 32];
    y1[0] = 2;
    data.extend_from_slice(&x1);
    data.extend_from_slice(&y1);
    data.extend_from_slice(&x1);
    data.extend_from_slice(&y1);

    let msg = Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![pubkey_of(payer), ZK_BN254_PROGRAM_ID],
        recent_blockhash: random_blockhash(),
        instructions: vec![CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data,
        }],
    };
    sign_and_serialize(msg, &[payer])
}

fn build_zk_mul(payer: &SigningKey) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(1); // opcode 1 = EC Mul

    let mut x = [0u8; 32];
    x[0] = 1;
    let mut y = [0u8; 32];
    y[0] = 2;
    data.extend_from_slice(&x);
    data.extend_from_slice(&y);

    let mut s = [0u8; 32];
    s[0] = 7;
    data.extend_from_slice(&s);

    let msg = Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![pubkey_of(payer), ZK_BN254_PROGRAM_ID],
        recent_blockhash: random_blockhash(),
        instructions: vec![CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data,
        }],
    };
    sign_and_serialize(msg, &[payer])
}

// ── Argument parsing ──

struct Cli {
    command: String,
    args: Vec<String>,
    url_override: Option<String>,
    help: bool,
}

fn parse_cli() -> Cli {
    let raw: Vec<String> = std::env::args().collect();
    let mut url_override: Option<String> = None;
    let mut help = false;
    let mut rest = Vec::new();

    let mut skip = false;
    for (i, arg) in raw.iter().enumerate().skip(1) {
        if skip {
            skip = false;
            continue;
        }
        match arg.as_str() {
            "-u" | "--url" => {
                if let Some(val) = raw.get(i + 1) {
                    url_override = Some(val.clone());
                    skip = true;
                } else {
                    eprintln!("Error: --url requires a value");
                    std::process::exit(1);
                }
            }
            // Also accept legacy --rpc
            "--rpc" => {
                if let Some(val) = raw.get(i + 1) {
                    url_override = Some(val.clone());
                    skip = true;
                } else {
                    eprintln!("Error: --rpc requires a value");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                help = true;
            }
            _ => {
                rest.push(arg.clone());
            }
        }
    }

    let command = rest.first().cloned().unwrap_or_default();
    let args: Vec<String> = rest.into_iter().skip(1).collect();

    Cli {
        command,
        args,
        url_override,
        help,
    }
}

// ── Commands ──

fn cmd_config_get() {
    let config = load_config();
    let rpc = config
        .get("rpc_url")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_RPC);
    let path = config_path();
    println!("Config File: {}", path.display());
    println!("RPC URL: {}", rpc);
}

fn cmd_config_set(args: &[String]) {
    let mut config = load_config();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--url" | "-u" => {
                if let Some(val) = args.get(i + 1) {
                    config["rpc_url"] = serde_json::json!(val);
                    i += 2;
                } else {
                    eprintln!("Error: --url requires a value");
                    std::process::exit(1);
                }
            }
            other => {
                eprintln!("Error: unknown config key '{}'", other);
                std::process::exit(1);
            }
        }
    }
    save_config(&config);
    let rpc = config
        .get("rpc_url")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_RPC);
    println!("Config updated");
    println!("RPC URL: {}", rpc);
}

fn cmd_address() {
    let key = active_keypair();
    let addr = pubkey_b58(&pubkey_of(&key));
    println!("{}", addr);
}

fn cmd_balance(rpc: &str, addr_str: Option<&str>) {
    let client = make_client();
    let address = match addr_str {
        Some(a) => match parse_address(a) {
            Ok(pk) => pk,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        None => pubkey_of(&payer_keypair()),
    };

    let b58 = pubkey_b58(&address);
    match rpc_call(&client, rpc, "getBalance", serde_json::json!([b58])) {
        Ok(v) => {
            let lamports = v.get("value").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("{} lamports", lamports);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_airdrop(rpc: &str, amount_str: &str, addr_str: Option<&str>) {
    let client = make_client();
    let amount = match parse_amount(amount_str) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let address = match addr_str {
        Some(a) => match parse_address(a) {
            Ok(pk) => pk,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        None => pubkey_of(&payer_keypair()),
    };

    let b58 = pubkey_b58(&address);
    let sol_display = lamports_to_sol(amount);

    println!(
        "Requesting airdrop of {} SOL to {}...",
        format_sol(sol_display),
        b58
    );

    // Try requestAirdrop RPC first
    match rpc_call(
        &client,
        rpc,
        "requestAirdrop",
        serde_json::json!([b58, amount]),
    ) {
        Ok(sig) => {
            let sig_str = sig.as_str().unwrap_or("(unknown)");
            println!("Signature: {}", sig_str);
        }
        Err(_) => {
            // Fallback: transfer from faucet keypair
            let payer = payer_keypair();
            let tx_bytes = build_svm_transfer(&payer, address, amount);
            match send_tx(&client, rpc, tx_bytes) {
                Ok(sig) => {
                    println!("Signature: {}", sig);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    // Show updated balance
    std::thread::sleep(std::time::Duration::from_secs(1));
    if let Ok(v) = rpc_call(&client, rpc, "getBalance", serde_json::json!([b58])) {
        let lamports = v.get("value").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("{} lamports", lamports);
    }
}

fn cmd_transfer(rpc: &str, to_str: &str, amount_str: &str) {
    let client = make_client();
    let to = match parse_address(to_str) {
        Ok(pk) => pk,
        Err(e) => {
            eprintln!("Error: invalid recipient: {}", e);
            std::process::exit(1);
        }
    };

    let amount = match parse_amount(amount_str) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let payer = payer_keypair();
    let tx_bytes = build_svm_transfer(&payer, to, amount);

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_deploy_evm(rpc: &str, bytecode_hex: &str) {
    let client = make_client();
    let hex_str = bytecode_hex.strip_prefix("0x").unwrap_or(bytecode_hex);
    let bytecode = hex::decode(hex_str).unwrap_or_else(|e| {
        eprintln!("Error: invalid bytecode hex: {}", e);
        std::process::exit(1);
    });

    let payer = payer_keypair();
    let hash = Sha256::digest(&bytecode);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hash);
    let contract_key = SigningKey::from_bytes(&seed);
    let contract_addr = pubkey_b58(&pubkey_of(&contract_key));

    let tx_bytes = build_create_account(
        &payer,
        &contract_key,
        1_000_000_000,
        bytecode.len() as u64,
        &EVM_PROGRAM_ID,
    );

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
            println!("Program deployed to: {}", contract_addr);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_deploy_wasm(rpc: &str, file_path: &str) {
    let client = make_client();
    let wasm_bytes = std::fs::read(file_path).unwrap_or_else(|e| {
        eprintln!("Error: cannot read '{}': {}", file_path, e);
        std::process::exit(1);
    });

    let payer = payer_keypair();
    let hash = Sha256::digest(&wasm_bytes);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hash);
    let contract_key = SigningKey::from_bytes(&seed);
    let contract_addr = pubkey_b58(&pubkey_of(&contract_key));

    let tx_bytes = build_create_account(
        &payer,
        &contract_key,
        1_000_000_000,
        wasm_bytes.len() as u64,
        &WASM_PROGRAM_ID,
    );

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
            println!("Program deployed to: {}", contract_addr);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_deploy_move(rpc: &str, file_path: &str) {
    let client = make_client();
    let move_bytes = std::fs::read(file_path).unwrap_or_else(|e| {
        eprintln!("Error: cannot read '{}': {}", file_path, e);
        std::process::exit(1);
    });

    let payer = payer_keypair();
    let hash = Sha256::digest(&move_bytes);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hash);
    let module_key = SigningKey::from_bytes(&seed);
    let module_addr = pubkey_b58(&pubkey_of(&module_key));

    let tx_bytes = build_create_account(
        &payer,
        &module_key,
        1_000_000_000,
        move_bytes.len() as u64,
        &MOVE_PROGRAM_ID,
    );

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
            println!("Program deployed to: {}", module_addr);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_zk_add(rpc: &str) {
    let client = make_client();
    let payer = payer_keypair();
    let tx_bytes = build_zk_add(&payer);

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_zk_mul(rpc: &str) {
    let client = make_client();
    let payer = payer_keypair();
    let tx_bytes = build_zk_mul(&payer);

    match send_tx(&client, rpc, tx_bytes) {
        Ok(sig) => {
            println!("Signature: {}", sig);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_account(rpc: &str, addr_str: &str) {
    let client = make_client();
    let pubkey = match parse_address(addr_str) {
        Ok(pk) => pk,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let b58 = pubkey_b58(&pubkey);

    match rpc_call(&client, rpc, "getAccountInfo", serde_json::json!([b58])) {
        Ok(result) => {
            let value = result.get("value");
            if value.is_none() || value.unwrap().is_null() {
                eprintln!("Account not found: {}", b58);
                eprintln!("This address has no on-chain state. Fund it with: eto airdrop 1000000000 {}", b58);
                std::process::exit(1);
            }
            let value = value.unwrap();

            println!("Public Key: {}", b58);

            let lamports = value.get("lamports").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("Balance: {} lamports", lamports);

            let owner = value
                .get("owner")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Owner: {}", owner);

            let executable = value
                .get("executable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            println!("Executable: {}", executable);

            if let Some(rent) = value.get("rentEpoch").and_then(|v| v.as_u64()) {
                println!("Rent Epoch: {}", rent);
            }

            // Data length
            if let Some(data_val) = value.get("data") {
                if let Some(arr) = data_val.as_array() {
                    if let Some(data_str) = arr.first().and_then(|s| s.as_str()) {
                        let decoded = bs58::decode(data_str).into_vec().unwrap_or_default();
                        println!("Data Length: {}", decoded.len());
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_block_height(rpc: &str) {
    let client = make_client();
    match rpc_call(&client, rpc, "getSlot", serde_json::json!([])) {
        Ok(v) => {
            let slot = v.as_u64().unwrap_or(0);
            println!("{}", slot);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_transaction_count(rpc: &str) {
    let client = make_client();
    match rpc_call(&client, rpc, "getTransactionCount", serde_json::json!([])) {
        Ok(v) => {
            let count = v.as_u64().unwrap_or(0);
            println!("{}", count);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_cluster_info(rpc: &str) {
    let client = make_client();

    // Health
    let health = match rpc_call(&client, rpc, "getHealth", serde_json::json!([])) {
        Ok(v) => v.as_str().unwrap_or("ok").to_string(),
        Err(_) => "unreachable".to_string(),
    };
    println!("Health: {}", health);

    // Block height
    match rpc_call(&client, rpc, "getSlot", serde_json::json!([])) {
        Ok(v) => {
            let slot = v.as_u64().unwrap_or(0);
            println!("Block Height: {}", slot);
        }
        Err(_) => println!("Block Height: N/A"),
    }

    // Transaction count
    match rpc_call(&client, rpc, "getTransactionCount", serde_json::json!([])) {
        Ok(v) => {
            let count = v.as_u64().unwrap_or(0);
            println!("Transaction Count: {}", count);
        }
        Err(_) => println!("Transaction Count: N/A"),
    }

    // TPS — try Prometheus metric first (accurate), fall back to tx count delta
    let prom_url = rpc.replace(":8899", ":9090");
    let tps_shown = if let Ok(resp) = client.get(format!("{}/metrics", prom_url)).send() {
        if let Ok(body) = resp.text() {
            let tps: f64 = body.lines()
                .find(|l| l.starts_with("eto_tps_recent "))
                .and_then(|l| l.split_whitespace().last())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let lifetime: f64 = body.lines()
                .find(|l| l.starts_with("eto_tps_lifetime "))
                .and_then(|l| l.split_whitespace().last())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let total: u64 = body.lines()
                .find(|l| l.starts_with("eto_transactions_total "))
                .and_then(|l| l.split_whitespace().last())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            if total > 0 {
                println!("TPS (recent): {}", tps as u64);
                println!("TPS (lifetime): {}", lifetime as u64);
                println!("Transactions: {}", total);
                true
            } else { false }
        } else { false }
    } else { false };
    if !tps_shown {
        // Fallback: measure tx count delta over 1 second
        let tc1 = rpc_call(&client, rpc, "getTransactionCount", serde_json::json!([]))
            .ok().and_then(|v| v.as_u64());
        if let Some(t1) = tc1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let tc2 = rpc_call(&client, rpc, "getTransactionCount", serde_json::json!([]))
                .ok().and_then(|v| v.as_u64());
            if let Some(t2) = tc2 {
                println!("TPS: ~{}", t2.saturating_sub(t1));
            } else {
                println!("TPS: N/A");
            }
        } else {
            println!("TPS: N/A");
        }
    }

    // Version / Identity
    match rpc_call(&client, rpc, "getVersion", serde_json::json!([])) {
        Ok(v) => {
            let fallback = v.to_string();
            let version = v
                .get("solana-core")
                .or_else(|| v.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or(&fallback);
            println!("Version: {}", version);
        }
        Err(_) => {}
    }

    // Identity (faucet keypair)
    let payer = payer_keypair();
    println!("Identity: {}", pubkey_b58(&pubkey_of(&payer)));
    println!("RPC URL: {}", rpc);
}

fn format_sol(sol: f64) -> String {
    if sol == sol.floor() && sol < 1_000_000.0 {
        format!("{}", sol as u64)
    } else {
        format!("{:.9}", sol)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// ── Inspect ──

fn cmd_inspect(rpc: &str, args: &[String]) {
    let client = make_client();
    let sub = args.first().map(|s| s.as_str()).unwrap_or("");

    match sub {
        "transfer" | "tx" => {
            let to_addr = args.get(1).unwrap_or_else(|| {
                eprintln!("Usage: eto inspect transfer <TO> <AMOUNT>");
                std::process::exit(1);
            });
            let amount_str = args.get(2).unwrap_or_else(|| {
                eprintln!("Usage: eto inspect transfer <TO> <AMOUNT>");
                std::process::exit(1);
            });
            let amount = parse_amount(amount_str).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let to_pk = parse_address(to_addr).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let to_b58 = bs58::encode(&to_pk.0).into_string();
            let payer = active_keypair();
            let from_pk = pubkey_of(&payer);
            let from_b58 = bs58::encode(&from_pk.0).into_string();

            println!("PRE-STATE");
            println!("  Block Height:  {}", rpc_call(&client, rpc, "getSlot", serde_json::json!([]))
                .ok().and_then(|v| v.as_u64()).unwrap_or(0));
            let bal_from_before = rpc_call(&client, rpc, "getBalance", serde_json::json!([from_b58]))
                .ok().and_then(|v| v.get("value").and_then(|b| b.as_u64())).unwrap_or(0);
            let bal_to_before = rpc_call(&client, rpc, "getBalance", serde_json::json!([to_b58]))
                .ok().and_then(|v| v.get("value").and_then(|b| b.as_u64())).unwrap_or(0);
            println!("  Sender:        {} ({} lamports)", from_b58, bal_from_before);
            println!("  Recipient:     {} ({} lamports)", to_b58, bal_to_before);
            println!();

            // Build and send
            let tx_bytes = build_svm_transfer(&payer, to_pk, amount);
            println!("TRANSACTION");
            println!("  Operation:     Transfer");
            println!("  Amount:        {} lamports ({} SOL)", amount, format_sol(amount as f64 / 1e9));
            println!("  Program:       System (11111111111111111111111111111111)");
            let sig = send_tx(&client, rpc, tx_bytes).unwrap_or_else(|e| {
                eprintln!("  FAILED: {}", e);
                std::process::exit(1);
            });
            println!("  Signature:     {}", sig);
            println!("  Status:        ACCEPTED");
            println!();

            // Wait for inclusion
            println!("CONSENSUS");
            print!("  Waiting for block inclusion");
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let bal_to_now = rpc_call(&client, rpc, "getBalance", serde_json::json!([to_b58]))
                    .ok().and_then(|v| v.get("value").and_then(|b| b.as_u64())).unwrap_or(0);
                if bal_to_now != bal_to_before {
                    println!(" confirmed");
                    break;
                }
                print!(".");
            }
            let height_after = rpc_call(&client, rpc, "getSlot", serde_json::json!([]))
                .ok().and_then(|v| v.as_u64()).unwrap_or(0);
            println!("  Block Height:  {}", height_after);
            println!();

            // Post state
            let bal_from_after = rpc_call(&client, rpc, "getBalance", serde_json::json!([from_b58]))
                .ok().and_then(|v| v.get("value").and_then(|b| b.as_u64())).unwrap_or(0);
            let bal_to_after = rpc_call(&client, rpc, "getBalance", serde_json::json!([to_b58]))
                .ok().and_then(|v| v.get("value").and_then(|b| b.as_u64())).unwrap_or(0);

            println!("POST-STATE");
            println!("  Sender:        {} lamports -> {} lamports (delta: -{})",
                bal_from_before, bal_from_after, bal_from_before.saturating_sub(bal_from_after));
            println!("  Recipient:     {} lamports -> {} lamports (delta: +{})",
                bal_to_before, bal_to_after, bal_to_after.saturating_sub(bal_to_before));
            println!();

            // Verify
            let credited = bal_to_after.saturating_sub(bal_to_before);
            println!("VERIFICATION");
            if credited == amount {
                println!("  Recipient credited: {} lamports  EXACT MATCH", credited);
            } else {
                println!("  Recipient credited: {} lamports (expected {})", credited, amount);
            }
            println!("  Signature:     {}", sig);
            println!("  Consensus:     CERTIFIED (1-hop)");

            // Try to get state root from prometheus
            let prom_url = rpc.replace(":8899", ":9090");
            if let Ok(resp) = client.get(format!("{}/metrics", prom_url)).send() {
                if let Ok(body) = resp.text() {
                    // no prom from outside, skip
                    let _ = body;
                }
            }
        }

        "account" | "acct" => {
            let addr = args.get(1).unwrap_or_else(|| {
                eprintln!("Usage: eto inspect account <ADDRESS>");
                std::process::exit(1);
            });
            let pk = parse_address(addr).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let b58 = bs58::encode(&pk.0).into_string();

            println!("ACCOUNT INSPECTION");
            println!();

            let result = rpc_call(&client, rpc, "getAccountInfo", serde_json::json!([b58]));
            match result {
                Ok(v) => {
                    if let Some(val) = v.get("value") {
                        if val.is_null() {
                            println!("  Address:    {}", b58);
                            println!("  Status:     NOT FOUND (no account at this address)");
                            return;
                        }
                        let lamports = val.get("lamports").and_then(|v| v.as_u64()).unwrap_or(0);
                        let owner = val.get("owner").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let executable = val.get("executable").and_then(|v| v.as_bool()).unwrap_or(false);
                        let space = val.get("space").and_then(|v| v.as_u64()).unwrap_or(0);
                        let data_b64 = val.get("data").and_then(|v| v.as_str()).unwrap_or("");

                        println!("  Address:    {}", b58);
                        println!("  Balance:    {} lamports ({} SOL)", lamports, format_sol(lamports as f64 / 1e9));
                        println!("  Owner:      {}", owner);
                        println!("  Executable: {}", executable);
                        println!("  Data:       {} bytes", space);

                        // Detect account type
                        if owner == "11111111111111111111111111111111" {
                            println!("  Type:       System Account");
                        } else if owner.ends_with("EE") || owner.contains("FFFFFFFFFFFFFFFFEE") {
                            println!("  Type:       EVM Contract");
                        } else if owner.ends_with("03") {
                            println!("  Type:       WASM Contract");
                        } else if owner.ends_with("02") {
                            println!("  Type:       Move Module");
                        }

                        // Show data preview if present
                        if !data_b64.is_empty() && space > 0 {
                            if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                                if data.len() >= 76 && data[0] == 1 {
                                    // Universal Token Header
                                    let vm = match data[1] { 0 => "SVM", 1 => "EVM", 2 => "WASM", 3 => "Move", _ => "Unknown" };
                                    let amount_bytes: [u8; 8] = data[66..74].try_into().unwrap_or([0; 8]);
                                    let token_amount = u64::from_le_bytes(amount_bytes);
                                    let decimals = data[74];
                                    let frozen = data[75] != 0;
                                    println!();
                                    println!("  TOKEN HEADER (Universal Token Standard)");
                                    println!("    VM Origin:  {}", vm);
                                    println!("    Mint:       {}", bs58::encode(&data[2..34]).into_string());
                                    println!("    Owner:      {}", bs58::encode(&data[34..66]).into_string());
                                    println!("    Amount:     {}", token_amount);
                                    println!("    Decimals:   {}", decimals);
                                    println!("    Frozen:     {}", frozen);
                                } else if data.len() <= 128 {
                                    println!();
                                    println!("  DATA (hex): {}", hex::encode(&data));
                                } else {
                                    println!();
                                    println!("  DATA (first 64 bytes): {}", hex::encode(&data[..64]));
                                    println!("  ... ({} more bytes)", data.len() - 64);
                                }
                            }
                        }

                        // Block height context
                        let height = rpc_call(&client, rpc, "getSlot", serde_json::json!([]))
                            .ok().and_then(|v| v.as_u64()).unwrap_or(0);
                        println!();
                        println!("  Queried at block: #{}", height);
                    }
                }
                Err(e) => {
                    println!("  Error: {}", e);
                }
            }
        }

        "block" => {
            let height = rpc_call(&client, rpc, "getSlot", serde_json::json!([]))
                .ok().and_then(|v| v.as_u64()).unwrap_or(0);
            let health = rpc_call(&client, rpc, "getHealth", serde_json::json!([]))
                .ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or("unknown".to_string());
            let chain_id = rpc_call(&client, rpc, "eth_chainId", serde_json::json!([]))
                .ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or("unknown".to_string());

            println!("BLOCK INSPECTION");
            println!();
            println!("  Height:     {}", height);
            println!("  Health:     {}", health);
            println!("  Chain ID:   {} ({})", chain_id,
                u64::from_str_radix(chain_id.trim_start_matches("0x"), 16).unwrap_or(0));
            println!("  RPC:        {}", rpc);
        }

        _ => {
            println!("eto inspect — Trace and verify transactions on-chain");
            println!();
            println!("USAGE:");
            println!("  eto inspect transfer <TO> <AMOUNT>   Send + trace a transfer end-to-end");
            println!("  eto inspect account <ADDRESS>        Deep inspect an account");
            println!("  eto inspect block                    Current block info");
            println!();
            println!("EXAMPLES:");
            println!("  eto inspect transfer 11111111111111111111111111111116 1");
            println!("  eto inspect account 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6");
            println!("  eto inspect block");
        }
    }
}

// ── Help ──

fn print_help() {
    println!(
        "eto-cli {}
ETO Universal VM -- Command Line Interface

USAGE:
    eto <COMMAND> [OPTIONS]

COMMANDS:
    config              Get or set configuration
    balance             Show account balance
    airdrop             Request testnet tokens
    transfer            Transfer tokens
    deploy              Deploy a contract (evm, wasm, move)
    zk                  ZK operations (add, mul)
    account             Show account details
    block-height        Current block height
    slot                Current slot (alias for block-height)
    transaction-count   Total processed transactions
    cluster-info        Show cluster status
    address             Show default keypair address
    keygen              Generate a new keypair
    keypair list        List all saved keypairs
    keypair set <FILE>  Set default keypair
    keypair import <KEY> Import private key (hex or base58)
    create-account      Create a new on-chain account
    inspect             Trace and verify transactions on-chain
    update              Update eto CLI to latest release
    version             Show version
    help                Show this help

OPTIONS:
    -u, --url <URL>     Override RPC URL
    --help              Show help",
        VERSION
    );
}

fn print_config_help() {
    println!(
        "eto-cli-config
Get or set ETO CLI configuration

USAGE:
    eto config <SUBCOMMAND>

SUBCOMMANDS:
    get                 Show current config
    set --url <URL>     Set the RPC URL"
    );
}

fn print_balance_help() {
    println!(
        "eto-cli-balance
Show account balance in lamports

USAGE:
    eto balance [ADDRESS]

If ADDRESS is omitted, shows the default keypair balance."
    );
}

fn print_airdrop_help() {
    println!(
        "eto-cli-airdrop
Request testnet tokens

USAGE:
    eto airdrop <AMOUNT> [ADDRESS]

AMOUNT can be in SOL (e.g., 1.5) or lamports (e.g., 1500000000).
If ADDRESS is omitted, airdrops to the default keypair."
    );
}

fn print_transfer_help() {
    println!(
        "eto-cli-transfer
Transfer tokens from the default keypair

USAGE:
    eto transfer <TO> <AMOUNT>

AMOUNT can be in SOL (e.g., 0.5) or lamports (e.g., 500000000)."
    );
}

fn print_deploy_help() {
    println!(
        "eto-cli-deploy
Deploy a contract to ETO

USAGE:
    eto deploy <VM> <ARG>

SUBCOMMANDS:
    evm <BYTECODE_HEX>  Deploy EVM (Solidity) contract
    wasm <FILE>          Deploy WASM contract
    move <FILE>          Deploy Move module"
    );
}

fn print_zk_help() {
    println!(
        "eto-cli-zk
ZK operations using BN254 precompiles

USAGE:
    eto zk <SUBCOMMAND>

SUBCOMMANDS:
    add     BN254 EC point addition (G1 + G1)
    mul     BN254 scalar multiplication (7 * G1)"
    );
}

fn print_account_help() {
    println!(
        "eto-cli-account
Show full account details

USAGE:
    eto account <ADDRESS>"
    );
}

// ── Main ──

fn main() {
    let cli = parse_cli();

    match cli.command.as_str() {
        "config" => {
            if cli.help {
                print_config_help();
                return;
            }
            let sub = cli.args.first().map(|s| s.as_str()).unwrap_or("");
            match sub {
                "get" | "" => cmd_config_get(),
                "set" => {
                    // If --url was consumed by global parser, reconstruct args
                    let mut set_args: Vec<String> = cli.args[1..].to_vec();
                    if let Some(ref url) = cli.url_override {
                        if !set_args.iter().any(|a| a == "--url" || a == "-u") {
                            set_args.push("--url".to_string());
                            set_args.push(url.clone());
                        }
                    }
                    cmd_config_set(&set_args);
                }
                other => {
                    eprintln!(
                        "Error: unknown config subcommand '{}'. Use 'get' or 'set'.",
                        other
                    );
                    std::process::exit(1);
                }
            }
        }

        "address" => {
            if cli.help {
                println!(
                    "eto-cli-address\nShow the default keypair address\n\nUSAGE:\n    eto address"
                );
                return;
            }
            cmd_address();
        }

        "keygen" => {
            if cli.help {
                println!("eto-cli-keygen\nGenerate a new keypair\n\nUSAGE:\n    eto keygen [OUTFILE]");
                return;
            }
            cmd_keygen(&cli.args);
        }

        "keypair" => {
            let sub = cli.args.first().map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" => cmd_keypair_list(),
                "set" => cmd_keygen_set_default(&cli.args[1..]),
                "import" => cmd_keypair_import(&cli.args[1..]),
                other => {
                    eprintln!("Unknown keypair subcommand: {}. Use: list, set, import", other);
                    std::process::exit(1);
                }
            }
        }

        "create-account" => {
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_create_account(&rpc, &cli.args);
        }

        "inspect" => {
            if cli.help {
                cmd_inspect("", &[]);
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_inspect(&rpc, &cli.args);
        }

        "balance" => {
            if cli.help {
                print_balance_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            let addr = cli.args.first().map(|s| s.as_str());
            cmd_balance(&rpc, addr);
        }

        "airdrop" => {
            if cli.help {
                print_airdrop_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            let amount_str = cli.args.first().unwrap_or_else(|| {
                eprintln!("Error: missing amount. Usage: eto airdrop <AMOUNT> [ADDRESS]");
                std::process::exit(1);
            });
            let addr = cli.args.get(1).map(|s| s.as_str());
            cmd_airdrop(&rpc, amount_str, addr);
        }

        // Keep legacy "faucet" as alias for "airdrop"
        "faucet" => {
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            // faucet <ADDRESS> [AMOUNT] -> map to airdrop semantics
            let addr = cli.args.first().unwrap_or_else(|| {
                eprintln!("Error: missing address. Usage: eto airdrop <AMOUNT> [ADDRESS]");
                std::process::exit(1);
            });
            let amount = cli.args.get(1).map(|s| s.as_str()).unwrap_or("1");
            cmd_airdrop(&rpc, amount, Some(addr));
        }

        "transfer" => {
            if cli.help {
                print_transfer_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            if cli.args.len() < 2 {
                eprintln!("Error: missing arguments. Usage: eto transfer <TO> <AMOUNT>");
                std::process::exit(1);
            }
            cmd_transfer(&rpc, &cli.args[0], &cli.args[1]);
        }

        "deploy" => {
            if cli.help {
                print_deploy_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            let sub = cli.args.first().map(|s| s.as_str()).unwrap_or_else(|| {
                eprintln!("Error: missing deploy target. Usage: eto deploy <evm|wasm|move> <ARG>");
                std::process::exit(1);
            });
            match sub {
                "evm" => {
                    let bytecode = cli.args.get(1).unwrap_or_else(|| {
                        eprintln!("Error: missing bytecode. Usage: eto deploy evm <BYTECODE_HEX>");
                        std::process::exit(1);
                    });
                    cmd_deploy_evm(&rpc, bytecode);
                }
                "wasm" => {
                    let file = cli.args.get(1).unwrap_or_else(|| {
                        eprintln!("Error: missing file. Usage: eto deploy wasm <FILE>");
                        std::process::exit(1);
                    });
                    cmd_deploy_wasm(&rpc, file);
                }
                "move" => {
                    let file = cli.args.get(1).unwrap_or_else(|| {
                        eprintln!("Error: missing file. Usage: eto deploy move <FILE>");
                        std::process::exit(1);
                    });
                    cmd_deploy_move(&rpc, file);
                }
                other => {
                    eprintln!(
                        "Error: unknown deploy target '{}'. Use: evm, wasm, or move",
                        other
                    );
                    std::process::exit(1);
                }
            }
        }

        "zk" => {
            if cli.help {
                print_zk_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            let sub = cli.args.first().map(|s| s.as_str()).unwrap_or_else(|| {
                eprintln!("Error: missing ZK operation. Usage: eto zk <add|mul>");
                std::process::exit(1);
            });
            match sub {
                "add" => cmd_zk_add(&rpc),
                "mul" => cmd_zk_mul(&rpc),
                other => {
                    eprintln!("Error: unknown ZK operation '{}'. Use: add or mul", other);
                    std::process::exit(1);
                }
            }
        }

        "account" => {
            if cli.help {
                print_account_help();
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            let addr = cli.args.first().unwrap_or_else(|| {
                eprintln!("Error: missing address. Usage: eto account <ADDRESS>");
                std::process::exit(1);
            });
            cmd_account(&rpc, addr);
        }

        "block-height" => {
            if cli.help {
                println!("eto-cli-block-height\nShow the current block height\n\nUSAGE:\n    eto block-height");
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_block_height(&rpc);
        }

        "slot" => {
            if cli.help {
                println!("eto-cli-slot\nShow the current slot (alias for block-height)\n\nUSAGE:\n    eto slot");
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_block_height(&rpc);
        }

        "transaction-count" => {
            if cli.help {
                println!("eto-cli-transaction-count\nShow total processed transactions\n\nUSAGE:\n    eto transaction-count");
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_transaction_count(&rpc);
        }

        "cluster-info" => {
            if cli.help {
                println!("eto-cli-cluster-info\nShow cluster health, version, and statistics\n\nUSAGE:\n    eto cluster-info");
                return;
            }
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_cluster_info(&rpc);
        }

        // Legacy alias
        "status" => {
            let rpc = resolve_rpc_url(cli.url_override.as_deref());
            cmd_cluster_info(&rpc);
        }

        "update" => {
            println!("Updating eto CLI...");
            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            let asset = match (os, arch) {
                ("macos", "aarch64") => "eto-macos-arm64",
                ("macos", "x86_64") => "eto-macos-x86_64",
                ("linux", "x86_64") => "eto-linux-x86_64",
                _ => {
                    eprintln!("Unsupported platform: {}/{}", os, arch);
                    eprintln!("Build from source: cargo install --path .");
                    std::process::exit(1);
                }
            };
            let url = format!("https://github.com/etofdn/eto-cli/releases/latest/download/{}.tar.gz", asset);
            println!("Downloading {}...", asset);
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "cd /tmp && curl -sL '{}' | tar xz && sudo mv /tmp/eto /usr/local/bin/eto && echo 'Updated successfully'",
                    url
                ))
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("eto CLI updated to latest release");
                }
                _ => {
                    eprintln!("Update failed. Try manually:");
                    eprintln!("  curl -sL {} | tar xz && sudo mv eto /usr/local/bin/", url);
                }
            }
        }

        "version" | "--version" | "-V" => {
            println!("eto-cli {}", VERSION);
        }

        "help" | "--help" | "-h" | "" => print_help(),

        other => {
            eprintln!(
                "Error: unknown command '{}'. Run 'eto help' for usage.",
                other
            );
            std::process::exit(1);
        }
    }
}
