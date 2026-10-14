use std::{collections::HashMap, sync::RwLock};

use axum::http::StatusCode;
use jsonwebtoken::{Validation, decode, decode_header};
use sha2::{Digest, Sha256};

use crate::{jwt_keys::JwtKeys, models::Claims, security::unix_now};

const SHARD_COUNT: usize = 16;

struct Entry {
    claims: Claims,
}

struct Shard {
    entries: HashMap<[u8; 32], Entry>,
    capacity: usize,
}

pub(crate) struct JwtVerificationCache {
    shards: [RwLock<Shard>; SHARD_COUNT],
}

impl JwtVerificationCache {
    pub(crate) fn new(capacity: usize) -> Self {
        let base_capacity = capacity / SHARD_COUNT;
        let extra = capacity % SHARD_COUNT;
        Self {
            shards: std::array::from_fn(|index| {
                RwLock::new(Shard {
                    entries: HashMap::new(),
                    capacity: base_capacity + usize::from(index < extra),
                })
            }),
        }
    }

    pub(crate) fn verify(
        &self,
        token: &str,
        keys: &JwtKeys,
        validation: &Validation,
    ) -> Result<Claims, StatusCode> {
        let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let now = unix_now()? as u64;
        if let Some(claims) = self.lookup(&digest, now) {
            return Ok(claims);
        }

        let header = decode_header(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
        let key = keys
            .verification_key(header.kid.as_deref())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let claims = decode::<Claims>(token, key, validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .claims;
        self.store(digest, claims.clone(), now);
        Ok(claims)
    }

    pub(crate) fn lookup(&self, digest: &[u8; 32], now: u64) -> Option<Claims> {
        let shard = &self.shards[shard_index(digest)];
        if let Ok(entries) = shard.read()
            && let Some(entry) = entries.entries.get(digest)
        {
            if entry.claims.exp > now {
                return Some(entry.claims.clone());
            }
        }

        if let Ok(mut entries) = shard.write() {
            entries.entries.remove(digest);
        }
        None
    }

    pub(crate) fn store(&self, digest: [u8; 32], claims: Claims, now: u64) {
        if claims.exp <= now {
            return;
        }
        let Ok(mut shard) = self.shards[shard_index(&digest)].write() else {
            return;
        };
        if shard.capacity == 0 {
            return;
        }

        shard.entries.retain(|_, entry| entry.claims.exp > now);
        if shard.entries.len() >= shard.capacity {
            if let Some(oldest) = shard
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.claims.exp)
                .map(|(digest, _)| *digest)
            {
                shard.entries.remove(&oldest);
            }
        }
        shard.entries.insert(digest, Entry { claims });
    }
}

pub(crate) fn shard_index(digest: &[u8; 32]) -> usize {
    let mut prefix = [0; 8];
    prefix.copy_from_slice(&digest[..8]);
    (u64::from_ne_bytes(prefix) as usize) % SHARD_COUNT
}
