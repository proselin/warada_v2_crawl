#![allow(dead_code)]

#[path = "../src/database.rs"]
mod database;
#[path = "../src/jwt_cache.rs"]
mod jwt_cache;
#[path = "../src/jwt_keys.rs"]
mod jwt_keys;
#[path = "../src/models.rs"]
mod models;
#[path = "../src/oauth.rs"]
mod oauth;
#[path = "../src/security.rs"]
mod security;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/tokens.rs"]
mod tokens;
#[path = "../src/verification.rs"]
mod verification;

use std::sync::Arc;

use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, Header, Validation, decode_header, encode};
use models::{Claims, PrincipalType};
use sha2::{Digest, Sha256};

#[test]
fn bearer_headers_require_exactly_one_token() {
    assert_eq!(security::bearer_token("Bearer abc.def"), Some("abc.def"));
    assert_eq!(security::bearer_token("bEaReR abc.def"), Some("abc.def"));
    assert_eq!(security::bearer_token("Basic abc"), None);
    assert_eq!(security::bearer_token("Bearer abc extra"), None);
}

#[test]
fn email_and_scope_inputs_are_normalized_and_validated() {
    assert_eq!(
        security::normalize_email("User@Example.com".to_owned()).unwrap(),
        "user@example.com"
    );
    assert!(security::normalize_email("not-an-email".to_owned()).is_err());
    assert_eq!(
        security::normalize_scopes(vec![
            "api:write".to_owned(),
            "api:read".to_owned(),
            "api:read".to_owned()
        ])
        .unwrap(),
        vec!["api:read", "api:write"]
    );
    assert!(security::normalize_scopes(vec!["bad scope".to_owned()]).is_err());
}

#[tokio::test]
async fn passwords_use_unique_salts_pepper_and_legacy_upgrade() {
    let pepper: Arc<[u8]> = Arc::from(b"test-pepper-with-at-least-32-bytes".as_slice());
    let hash = security::hash_password("correct horse battery staple".to_owned(), pepper.clone())
        .await
        .unwrap();
    let second_hash =
        security::hash_password("correct horse battery staple".to_owned(), pepper.clone())
            .await
            .unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert_ne!(hash, second_hash);
    assert!(matches!(
        security::verify_password(
            "correct horse battery staple".to_owned(),
            hash,
            pepper.clone()
        )
        .await
        .unwrap(),
        security::PasswordCheck::Valid
    ));
    assert!(matches!(
        security::verify_password(
            "correct horse battery staple".to_owned(),
            second_hash,
            Arc::from(b"different-pepper-also-more-than-32-bytes".as_slice())
        )
        .await
        .unwrap(),
        security::PasswordCheck::Invalid
    ));

    let legacy = security::hash_secret("legacy password".to_owned())
        .await
        .unwrap();
    assert!(matches!(
        security::verify_password("legacy password".to_owned(), legacy, pepper)
            .await
            .unwrap(),
        security::PasswordCheck::ValidNeedsRehash
    ));
}

#[test]
fn jwt_cache_expires_entries_and_bounds_each_shard() {
    let cache = jwt_cache::JwtVerificationCache::new(16);
    let mut digests = (0..)
        .map(|index| {
            let token = format!("token-{index}");
            let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
            digest
        })
        .filter(|digest| jwt_cache::shard_index(digest) == 0);
    let first = digests.next().unwrap();
    let second = digests.next().unwrap();
    let claims = |exp| Claims {
        sub: "user-1".to_owned(),
        iss: "issuer".to_owned(),
        aud: "audience".to_owned(),
        exp,
        iat: exp.saturating_sub(10),
        principal_type: PrincipalType::User,
        scope: Some("api:read".to_owned()),
    };

    cache.store(first, claims(200), 100);
    assert_eq!(cache.lookup(&first, 100).unwrap().sub, "user-1");
    assert!(cache.lookup(&first, 200).is_none());
    cache.store(first, claims(300), 100);
    cache.store(second, claims(400), 100);
    assert!(cache.lookup(&first, 100).is_none());
    assert!(cache.lookup(&second, 100).is_some());
}

#[test]
fn pkce_redirects_and_authorization_template_are_safe() {
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    assert!(oauth::valid_code_verifier(verifier));
    assert_eq!(oauth::pkce_challenge(verifier).len(), 43);
    assert_ne!(oauth::hash_code("first"), oauth::hash_code("second"));
    assert!(!oauth::valid_code_verifier("too-short"));

    assert!(oauth::valid_redirect_uri("https://app.example/callback"));
    assert!(oauth::valid_redirect_uri("http://localhost:3000/callback"));
    assert!(!oauth::valid_redirect_uri("http://app.example/callback"));
    assert!(!oauth::valid_redirect_uri(
        "https://app.example/callback#fragment"
    ));

    assert_eq!(
        oauth::render_template(
            "{{A}} {{B}}",
            &[("A", "{{B}}".to_owned()), ("B", "done".to_owned())]
        ),
        "{{B}} done"
    );
}

#[test]
fn authorization_policy_requires_pkce_and_gates_implicit_flow() {
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = oauth::pkce_challenge(verifier);
    assert!(
        oauth::validate_flow_policy("code", "state", Some(&challenge), Some("S256"), false).is_ok()
    );
    assert!(oauth::validate_flow_policy("code", "state", None, Some("S256"), false).is_err());
    assert!(
        oauth::validate_flow_policy("code", "state", Some(&challenge), Some("plain"), false)
            .is_err()
    );
    assert!(oauth::validate_flow_policy("token", "state", None, None, false).is_err());
    assert!(oauth::validate_flow_policy("token", "state", None, None, true).is_ok());
    assert!(
        oauth::validate_flow_policy("code", "", Some(&challenge), Some("S256"), false).is_err()
    );

    let login_page = include_str!("../assets/authorize.html");
    assert!(login_page.contains("name=\"code_challenge\""));
    assert!(login_page.contains("name=\"code_challenge_method\""));
}

#[test]
fn tokens_survive_a_signing_key_rollout_overlap() {
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let public_keys = fixture_root.join("jwt-keys");
    let old_keys = jwt_keys::JwtKeys::load(
        &public_keys,
        &fixture_root.join("jwt-private/old.pem"),
        "old".to_owned(),
        "old".to_owned(),
    )
    .unwrap();
    let new_keys = jwt_keys::JwtKeys::load(
        &public_keys,
        &fixture_root.join("jwt-private/new.pem"),
        "new".to_owned(),
        "old".to_owned(),
    )
    .unwrap();

    let old_token = tokens::issue_token(
        &old_keys,
        "https://issuer.test/",
        "test-api",
        300,
        "old-client".to_owned(),
        PrincipalType::Service,
        vec!["api:read".to_owned()],
    )
    .unwrap();
    let new_token = tokens::issue_token(
        &new_keys,
        "https://issuer.test/",
        "test-api",
        300,
        "new-client".to_owned(),
        PrincipalType::Service,
        vec!["api:write".to_owned()],
    )
    .unwrap();
    assert_eq!(
        decode_header(&old_token.access_token)
            .unwrap()
            .kid
            .as_deref(),
        Some("old")
    );
    assert_eq!(
        decode_header(&new_token.access_token)
            .unwrap()
            .kid
            .as_deref(),
        Some("new")
    );

    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_issuer(&["https://issuer.test/"]);
    validation.set_audience(&["test-api"]);
    let cache = jwt_cache::JwtVerificationCache::new(32);
    let old_claims = cache
        .verify(&old_token.access_token, &new_keys, &validation)
        .unwrap();
    let new_claims = cache
        .verify(&new_token.access_token, &new_keys, &validation)
        .unwrap();
    assert_eq!(old_claims.sub, "old-client");
    assert_eq!(new_claims.sub, "new-client");
    assert_eq!(old_claims.iss, "https://issuer.test/");
    assert_eq!(new_claims.aud, "test-api");
    assert_eq!(old_claims.scope.as_deref(), Some("api:read"));
    assert_eq!(new_claims.scope.as_deref(), Some("api:write"));
    assert!(old_claims.exp > old_claims.iat);
    assert_eq!(old_token.expires_in, 300);

    let now = security::unix_now().unwrap() as u64;
    let legacy_claims = Claims {
        sub: "legacy-client".to_owned(),
        iss: "https://issuer.test/".to_owned(),
        aud: "test-api".to_owned(),
        exp: now + 300,
        iat: now,
        principal_type: PrincipalType::Service,
        scope: Some("api:read".to_owned()),
    };
    let legacy_token = encode(
        &Header::new(Algorithm::RS256),
        &legacy_claims,
        old_keys.signing_key(),
    )
    .unwrap();
    assert_eq!(
        cache
            .verify(&legacy_token, &new_keys, &validation)
            .unwrap()
            .sub,
        "legacy-client"
    );

    let mut unknown_kid_header = Header::new(Algorithm::RS256);
    unknown_kid_header.kid = Some("retired".to_owned());
    let unknown_kid_token =
        encode(&unknown_kid_header, &legacy_claims, new_keys.signing_key()).unwrap();
    assert!(matches!(
        cache.verify(&unknown_kid_token, &new_keys, &validation),
        Err(StatusCode::UNAUTHORIZED)
    ));
}
