use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute_login_proof(username: &str, password: &str, nonce: &str) -> String {
    let input = format!("{}:{}:{}:reverz-wasm-pepper-v1", username, password, nonce);
    let hash = Sha256::digest(input.as_bytes());
    to_hex(&hash)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}