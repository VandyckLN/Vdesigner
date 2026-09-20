//! The updater fails silently when misconfigured: an empty public key or a
//! missing artifact flag still builds, still ships, and only reveals itself
//! when an update never arrives. These assertions turn that into a red test.

const CONFIG: &str = include_str!("../tauri.conf.json");

#[test]
fn the_updater_declares_a_public_key() {
    let config: serde_json::Value = serde_json::from_str(CONFIG).expect("tauri.conf.json inválido");
    let key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("faltou plugins.updater.pubkey");
    assert!(
        !key.trim().is_empty(),
        "a chave pública do atualizador está vazia"
    );
}

#[test]
fn the_updater_points_at_the_released_manifest() {
    let config: serde_json::Value = serde_json::from_str(CONFIG).expect("tauri.conf.json inválido");
    let endpoints = config["plugins"]["updater"]["endpoints"]
        .as_array()
        .expect("faltou plugins.updater.endpoints");
    assert_eq!(endpoints.len(), 1, "esperava um endpoint só");
    assert_eq!(
        endpoints[0].as_str().unwrap_or_default(),
        "https://raw.githubusercontent.com/VandyckLN/Vdesigner/main/updates/stable.json",
        "o atualizador precisa ler o manifesto liberado, não a última release"
    );
}

#[test]
fn the_bundle_emits_updater_artifacts() {
    let config: serde_json::Value = serde_json::from_str(CONFIG).expect("tauri.conf.json inválido");
    assert_eq!(
        config["bundle"]["createUpdaterArtifacts"].as_bool(),
        Some(true),
        "sem os artefatos do atualizador o CI não produz o .sig"
    );
}
