use crate::base::ConfigKey;
use crate::config::ProviderConfigStore;
use crate::utils::bytes_to_hex;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryIdentity {
    pub provider_id: String,
    pub provider_family: String,
    pub inventory_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryIdentityInput {
    pub provider_id: String,
    pub provider_family: String,
    pub public_inputs: BTreeMap<String, String>,
    pub secret_inputs: BTreeMap<String, String>,
}

impl InventoryIdentityInput {
    pub fn new(
        provider_id: impl Into<String>,
        provider_family: impl Into<String>,
    ) -> InventoryIdentityInput {
        InventoryIdentityInput {
            provider_id: provider_id.into(),
            provider_family: provider_family.into(),
            public_inputs: BTreeMap::new(),
            secret_inputs: BTreeMap::new(),
        }
    }

    pub fn with_public(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> InventoryIdentityInput {
        self.public_inputs.insert(key.into(), value.into());
        self
    }

    pub fn with_secret(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> InventoryIdentityInput {
        self.secret_inputs.insert(key.into(), value.into());
        self
    }

    pub fn into_identity(self) -> Result<InventoryIdentity> {
        let InventoryIdentityInput {
            provider_id,
            provider_family,
            public_inputs,
            secret_inputs,
        } = self;
        let payload = serde_json::json!({
            "provider_family": provider_family,
            "public_inputs": public_inputs,
            "secret_inputs": secret_inputs,
        });
        let digest = Sha256::digest(serde_json::to_vec(&payload)?);
        Ok(InventoryIdentity {
            provider_id,
            provider_family,
            inventory_key: bytes_to_hex(digest),
        })
    }
}

pub fn default_inventory_identity(
    provider_id: &str,
    provider_family: &str,
    config_keys: &[ConfigKey],
    config: &dyn ProviderConfigStore,
) -> InventoryIdentityInput {
    let mut input = InventoryIdentityInput::new(provider_id, provider_family);

    for key in config_keys {
        if key.secret {
            if let Some(value) = config_secret_value(config, &key.name) {
                input = input.with_secret(&key.name, value);
            }
        } else if let Some(value) = config_param_value(config, &key.name) {
            input = input.with_public(&key.name, value);
        }
    }

    input
}

pub fn default_inventory_configured(
    config_keys: &[ConfigKey],
    config: &dyn ProviderConfigStore,
) -> bool {
    config_keys.iter().all(|key| {
        if !key.required {
            return true;
        }
        if key.default.is_some() {
            return true;
        }
        if key.secret {
            config.get_secret_value(&key.name).is_ok()
        } else {
            config.get_param_value(&key.name).is_ok()
        }
    })
}

fn config_param_value(config: &dyn ProviderConfigStore, key: &str) -> Option<String> {
    config
        .get_param_value(key)
        .ok()
        .and_then(|value| normalize_json_value(&value))
}

fn config_secret_value(config: &dyn ProviderConfigStore, key: &str) -> Option<String> {
    config
        .get_secret_value(key)
        .ok()
        .and_then(|value| normalize_json_value(&value))
}

fn normalize_json_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) if value.is_empty() => None,
        serde_json::Value::String(value) => Some(value.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryModel {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    pub recommended: bool,
}
