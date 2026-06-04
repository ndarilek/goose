use anyhow::Result;
use futures::future::BoxFuture;
use std::path::PathBuf;

use super::inventory::InventoryIdentityInput;
use crate::config::ExtensionConfig;
use goose_providers::base::ProviderMetadata;
use goose_providers::inventory as provider_inventory;

use super::mode::GooseProvider;
use goose_providers::model::ModelConfig;

pub trait ProviderDef: Send + Sync {
    type Provider: GooseProvider + 'static;

    fn metadata() -> ProviderMetadata
    where
        Self: Sized;

    fn from_env(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized;

    fn from_env_with_working_dir(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
        _working_dir: PathBuf,
    ) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized,
    {
        Self::from_env(model, extensions)
    }

    fn supports_inventory_refresh() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn inventory_identity() -> Result<InventoryIdentityInput>
    where
        Self: Sized,
    {
        let metadata = Self::metadata();
        let runtime = crate::providers::runtime::global_provider_runtime();
        let input = provider_inventory::default_inventory_identity(
            &metadata.name,
            &metadata.name,
            &metadata.config_keys,
            runtime.config.as_ref(),
        );
        Ok(InventoryIdentityInput {
            provider_id: input.provider_id,
            provider_family: input.provider_family,
            public_inputs: input.public_inputs,
            secret_inputs: input.secret_inputs,
        })
    }

    fn inventory_configured() -> bool
    where
        Self: Sized,
    {
        let metadata = Self::metadata();
        let runtime = crate::providers::runtime::global_provider_runtime();
        provider_inventory::default_inventory_configured(
            &metadata.config_keys,
            runtime.config.as_ref(),
        )
    }
}
