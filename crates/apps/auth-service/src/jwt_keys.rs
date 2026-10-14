use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::Path,
    sync::Arc,
};

use jsonwebtoken::{DecodingKey, EncodingKey};

pub(crate) struct JwtKeys {
    active_key_id: String,
    legacy_key_id: String,
    signing_key: EncodingKey,
    verification_keys: HashMap<String, Arc<DecodingKey>>,
}

impl JwtKeys {
    pub(crate) fn load(
        public_keys_dir: &Path,
        private_key_path: &Path,
        active_key_id: String,
        legacy_key_id: String,
    ) -> Result<Self, Box<dyn Error>> {
        if !valid_key_id(&active_key_id) || !valid_key_id(&legacy_key_id) {
            return Err(IoError::new(ErrorKind::InvalidInput, "invalid JWT key id").into());
        }

        let mut verification_keys = HashMap::new();
        for entry in fs::read_dir(public_keys_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("pem") {
                continue;
            }
            let Some(key_id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                return Err(
                    IoError::new(ErrorKind::InvalidInput, "invalid public key filename").into(),
                );
            };
            if !valid_key_id(key_id) {
                return Err(
                    IoError::new(ErrorKind::InvalidInput, "invalid JWT public key id").into(),
                );
            }
            let pem = fs::read(&path)?;
            let key = DecodingKey::from_rsa_pem(&pem)?;
            if verification_keys
                .insert(key_id.to_owned(), Arc::new(key))
                .is_some()
            {
                return Err(
                    IoError::new(ErrorKind::InvalidInput, "duplicate JWT public key id").into(),
                );
            }
        }

        if !verification_keys.contains_key(&active_key_id)
            || !verification_keys.contains_key(&legacy_key_id)
        {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "active and legacy JWT key ids must exist in the public key directory",
            )
            .into());
        }

        let private_pem = fs::read(private_key_path)?;
        let signing_key = EncodingKey::from_rsa_pem(&private_pem)?;
        Ok(Self {
            active_key_id,
            legacy_key_id,
            signing_key,
            verification_keys,
        })
    }

    pub(crate) fn active_key_id(&self) -> &str {
        &self.active_key_id
    }

    pub(crate) fn signing_key(&self) -> &EncodingKey {
        &self.signing_key
    }

    pub(crate) fn verification_key(&self, key_id: Option<&str>) -> Option<&DecodingKey> {
        let key_id = key_id.unwrap_or(&self.legacy_key_id);
        self.verification_keys.get(key_id).map(Arc::as_ref)
    }
}

fn valid_key_id(key_id: &str) -> bool {
    !key_id.is_empty()
        && key_id.len() <= 100
        && key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
