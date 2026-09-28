use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Expected config path and downloaded asset directory".into());
    }
    let config: serde_json::Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or("Missing updater key")?;
    let key = PublicKey::decode(&String::from_utf8(STANDARD.decode(key.trim())?)?)?;
    let directory = PathBuf::from(&args[2]);
    let mut verified = 0;
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid filename")?;
        if let Some(package) = name.strip_suffix(".sig") {
            let encoded = fs::read_to_string(&path)?;
            let signature =
                Signature::decode(&String::from_utf8(STANDARD.decode(encoded.trim())?)?)?;
            let mut bytes = fs::read(directory.join(package))?;
            key.verify(&bytes, &signature, true)?;
            if bytes.is_empty() {
                return Err("Empty signed package".into());
            }
            bytes[0] ^= 1;
            if key.verify(&bytes, &signature, true).is_ok() {
                return Err("Modified package accepted".into());
            }
            println!("Verified signature and tamper rejection: {package}");
            verified += 1;
        }
    }
    if verified != 5 {
        return Err(format!("Expected 5 signatures, found {verified}").into());
    }
    Ok(())
}
