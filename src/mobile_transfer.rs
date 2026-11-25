//! Offline mobile import: LMQR1:<transfer-id>:<part>:<count>:<base64-zlib-chunk>.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
pub struct MobileSettings {
    pub format: String,
    pub version: u32,
    pub username: String,
    pub avatar: u8,
    pub backup: crate::backup::Backup,
}

pub fn encode(settings: &MobileSettings) -> Result<Vec<String>, String> {
    settings.backup.validate()?;
    let json = serde_json::to_vec(settings).map_err(|e| e.to_string())?;
    let mut compressor =
        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    compressor.write_all(&json).map_err(|e| e.to_string())?;
    let data = URL_SAFE_NO_PAD.encode(compressor.finish().map_err(|e| e.to_string())?);
    if data.len() > 128 * 1024 {
        return Err(
            "This inventory is too large for QR transfer. Use Export backup instead.".into(),
        );
    }
    let mut random = [0u8; 16];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut random);
    let id = URL_SAFE_NO_PAD.encode(random);
    let count = data.len().div_ceil(700);
    Ok(data
        .as_bytes()
        .chunks(700)
        .enumerate()
        .map(|(i, part)| {
            format!(
                "LMQR1:{id}:{}:{count}:{}",
                i + 1,
                std::str::from_utf8(part).unwrap()
            )
        })
        .collect())
}

/// Reference decoder for the planned mobile implementation. Accepts any scan order.
pub fn decode(parts: &[String]) -> Result<MobileSettings, String> {
    if parts.is_empty() || parts.len() > 188 {
        return Err("Invalid QR transfer size".into());
    }
    let mut ordered = std::collections::BTreeMap::new();
    let mut expected = None;
    for part in parts {
        if part.len() > 800 {
            return Err("Invalid QR page size".into());
        }
        let fields: Vec<_> = part.splitn(5, ':').collect();
        if fields.len() != 5 || fields[0] != "LMQR1" {
            return Err("Unsupported QR format".into());
        }
        let index = fields[2].parse::<usize>().map_err(|_| "Invalid QR page")?;
        let total = fields[3].parse::<usize>().map_err(|_| "Invalid QR count")?;
        if total != parts.len() || index == 0 || index > total || fields[1].len() != 22 {
            return Err("Scan every page of the same transfer".into());
        }
        if expected.is_some_and(|id| id != fields[1]) || ordered.insert(index, fields[4]).is_some()
        {
            return Err("Mixed transfers or duplicate QR pages".into());
        }
        expected = Some(fields[1]);
    }
    let encoded: String = ordered.values().copied().collect();
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| "Invalid QR data")?;
    let mut json = Vec::new();
    flate2::read::ZlibDecoder::new(compressed.as_slice())
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut json)
        .map_err(|_| "Corrupt QR transfer")?;
    if json.len() > 32 * 1024 * 1024 {
        return Err("QR payload too large".into());
    }
    let settings: MobileSettings =
        serde_json::from_slice(&json).map_err(|_| "Invalid mobile settings")?;
    if settings.format != "labmanager.mobile" || settings.version != 1 {
        return Err("Unsupported mobile settings version".into());
    }
    settings.backup.validate()?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    #[test]
    fn mobile_transfer_round_trip_and_invalid_pages() {
        let settings = super::MobileSettings {
            format: "labmanager.mobile".into(),
            version: 1,
            username: "alice".into(),
            avatar: 2,
            backup: crate::backup::Backup {
                version: 1,
                created: 1,
                devices: vec![crate::model::Server::new("192.0.2.1".parse().unwrap())],
                monitoring: Default::default(),
                icons: Default::default(),
            },
        };
        let mut parts = super::encode(&settings).unwrap();
        parts.reverse();
        let restored = super::decode(&parts).unwrap();
        assert_eq!(restored.username, "alice");
        assert_eq!(restored.backup.devices, settings.backup.devices);
        parts.push(parts[0].clone());
        assert!(super::decode(&parts).is_err());
        assert!(super::decode(&[]).is_err());
    }
}
