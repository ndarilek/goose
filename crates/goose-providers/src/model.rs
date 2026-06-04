use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use utoipa::ToSchema;

use crate::config::{ProviderConfigError, ProviderConfigStore, ProviderRuntime};

pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingEffort {
    Off,
    Low,
    Medium,
    High,
    Max,
}

impl FromStr for ThinkingEffort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" | "disabled" | "none" => Ok(Self::Off),
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "max" | "xhigh" => Ok(Self::Max),
            other => Err(format!("unknown thinking effort: '{other}'")),
        }
    }
}

impl fmt::Display for ThinkingEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Max => write!(f, "max"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PredefinedModel {
    name: String,
    #[serde(default)]
    context_limit: Option<usize>,
    #[serde(default)]
    request_params: Option<HashMap<String, Value>>,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration key '{0}' not found")]
    EnvVarMissing(String),
    #[error("Invalid value for '{0}': '{1}' - {2}")]
    InvalidValue(String, String, String),
    #[error("Value for '{0}' is out of valid range: {1}")]
    InvalidRange(String, String),
}

pub use ConfigError as ModelConfigError;

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ModelConfig {
    pub model_name: String,
    pub context_limit: Option<usize>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub toolshim: bool,
    pub toolshim_model: Option<String>,
    #[serde(skip)]
    pub fast_model_config: Option<Box<ModelConfig>>,
    /// Provider-specific request parameters (e.g., anthropic_beta headers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_params: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
}

impl<'de> Deserialize<'de> for ModelConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawModelConfig {
            model_name: String,
            context_limit: Option<usize>,
            temperature: Option<f32>,
            max_tokens: Option<i32>,
            toolshim: bool,
            toolshim_model: Option<String>,
            #[serde(default)]
            fast_model_config: Option<Box<ModelConfig>>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            request_params: Option<HashMap<String, Value>>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            reasoning: Option<bool>,
        }

        let raw = RawModelConfig::deserialize(deserializer)?;
        let mut config = Self {
            model_name: raw.model_name,
            context_limit: raw.context_limit,
            temperature: raw.temperature,
            max_tokens: raw.max_tokens,
            toolshim: raw.toolshim,
            toolshim_model: raw.toolshim_model,
            fast_model_config: raw.fast_model_config,
            request_params: raw.request_params,
            reasoning: raw.reasoning,
        };
        config.normalize_effort_suffix();
        Ok(config)
    }
}

impl ModelConfig {
    pub fn new(model_name: &str) -> Result<Self, ConfigError> {
        let mut config = Self {
            model_name: model_name.to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        config.normalize_effort_suffix();
        Ok(config)
    }

    pub fn new_with_context_env(
        model_name: String,
        provider_name: &str,
        _context_env_var: Option<&str>,
    ) -> Result<Self, ConfigError> {
        Ok(Self::new(&model_name)?.with_canonical_limits(provider_name))
    }

    pub fn with_canonical_limits(mut self, provider_name: &str) -> Self {
        let canonical =
            crate::canonical::maybe_get_canonical_model(provider_name, &self.model_name).or_else(
                || {
                    let (base, _effort) = crate::utils::extract_reasoning_effort(&self.model_name);
                    if base != self.model_name {
                        crate::canonical::maybe_get_canonical_model(provider_name, &base)
                    } else {
                        None
                    }
                },
            );

        if let Some(canonical) = canonical {
            if self.context_limit.is_none() {
                self.context_limit = Some(canonical.limit.context);
            }
            if self.max_tokens.is_none() {
                self.max_tokens = canonical
                    .limit
                    .output
                    .filter(|&output| output < canonical.limit.context)
                    .map(|output| output as i32);
            }
            if self.reasoning.is_none() {
                self.reasoning = canonical.reasoning;
            }
        }

        self
    }

    fn validate_context_limit(limit: usize, key: &str) -> Result<usize, ConfigError> {
        if key == "GOOSE_CONTEXT_LIMIT" {
            if limit == 0 {
                return Err(ConfigError::InvalidRange(
                    key.to_string(),
                    "must be greater than 0".to_string(),
                ));
            }
        } else if limit < 4 * 1024 {
            return Err(ConfigError::InvalidRange(
                key.to_string(),
                "must be greater than 4K".to_string(),
            ));
        }

        Ok(limit)
    }

    fn parse_context_limit(value: Value, key: &str) -> Result<usize, ConfigError> {
        let limit = value_to_usize(value, key)?;
        Self::validate_context_limit(limit, key)
    }

    fn parse_max_tokens(value: Value, key: &str) -> Result<i32, ConfigError> {
        let tokens = value_to_i32(value, key)?;
        if tokens <= 0 {
            return Err(ConfigError::InvalidRange(
                key.to_string(),
                "must be greater than 0".to_string(),
            ));
        }
        Ok(tokens)
    }

    fn parse_temperature(value: Value, key: &str) -> Result<f32, ConfigError> {
        let temp = value_to_f32(value, key)?;
        if temp < 0.0 {
            return Err(ConfigError::InvalidRange(key.to_string(), temp.to_string()));
        }
        Ok(temp)
    }

    fn parse_toolshim(value: Value, key: &str) -> Result<bool, ConfigError> {
        value_to_bool(value, key)
    }

    fn parse_toolshim_model(value: Value, key: &str) -> Result<String, ConfigError> {
        let model = value_to_string(value, key)?;
        if model.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                key.to_string(),
                model,
                "cannot be empty if set".to_string(),
            ));
        }
        Ok(model)
    }

    pub fn with_context_limit(mut self, limit: Option<usize>) -> Self {
        if limit.is_some() {
            self.context_limit = limit;
        }
        self
    }

    pub fn with_temperature(mut self, temp: Option<f32>) -> Self {
        self.temperature = temp;
        self
    }

    pub fn with_max_tokens(mut self, tokens: Option<i32>) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn with_toolshim(mut self, toolshim: bool) -> Self {
        self.toolshim = toolshim;
        self
    }

    pub fn with_toolshim_model(mut self, model: Option<String>) -> Self {
        self.toolshim_model = model;
        self
    }

    pub fn with_fast(
        mut self,
        fast_model_name: &str,
        provider_name: &str,
    ) -> Result<Self, ConfigError> {
        if self.fast_model_config.is_none() {
            let fast_config =
                ModelConfig::new(fast_model_name)?.with_canonical_limits(provider_name);
            self.fast_model_config = Some(Box::new(fast_config));
        }
        Ok(self)
    }

    pub fn with_merged_request_params(mut self, params: HashMap<String, Value>) -> Self {
        merge_request_params_overriding(&mut self.request_params, params);
        self
    }

    pub fn use_fast_model(&self) -> Self {
        if let Some(fast_config) = &self.fast_model_config {
            *fast_config.clone()
        } else {
            self.clone()
        }
    }

    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }

    pub fn is_openai_reasoning_model(&self) -> bool {
        crate::utils::is_openai_responses_model(&self.model_name)
    }

    pub fn is_reasoning_model(&self) -> bool {
        if let Some(reasoning) = self.reasoning {
            return reasoning;
        }

        self.is_openai_reasoning_model()
            || self.model_name.to_lowercase().contains("claude")
            || Self::is_gemini3_reasoning_model_name(&self.model_name)
    }

    fn is_gemini3_reasoning_model_name(model_name: &str) -> bool {
        let lower = model_name.to_lowercase();
        lower.starts_with("gemini-3") || lower.contains("/gemini-3") || lower.contains("-gemini-3")
    }

    pub fn max_output_tokens(&self) -> i32 {
        if let Some(tokens) = self.max_tokens {
            return tokens;
        }

        4_096
    }

    pub fn normalize_effort_suffix(&mut self) {
        if !self.is_openai_reasoning_model() {
            return;
        }
        let parts: Vec<&str> = self.model_name.split('-').collect();
        let last = match parts.last() {
            Some(l) => *l,
            None => return,
        };
        let effort = match last {
            "none" => ThinkingEffort::Off,
            "low" => ThinkingEffort::Low,
            "medium" => ThinkingEffort::Medium,
            "high" => ThinkingEffort::High,
            "xhigh" => ThinkingEffort::Max,
            _ => return,
        };
        self.model_name = parts[..parts.len() - 1].join("-");
        let has_explicit_effort = self
            .request_params
            .as_ref()
            .and_then(|p| p.get("thinking_effort"))
            .is_some();
        if !has_explicit_effort {
            let params = self.request_params.get_or_insert_with(HashMap::new);
            params.insert(
                "thinking_effort".to_string(),
                serde_json::json!(effort.to_string()),
            );
        }
    }

    pub fn thinking_effort(&self) -> Option<ThinkingEffort> {
        self.get_config_param::<String>("thinking_effort", "GOOSE_THINKING_EFFORT")
            .and_then(|s| s.parse::<ThinkingEffort>().ok())
    }

    fn legacy_gemini3_thinking_effort(value: &str) -> Option<ThinkingEffort> {
        match value.to_lowercase().as_str() {
            "low" => Some(ThinkingEffort::Low),
            "high" => Some(ThinkingEffort::High),
            _ => None,
        }
    }

    pub fn get_config_param<T: DeserializeOwned>(
        &self,
        request_key: &str,
        _config_key: &str,
    ) -> Option<T> {
        self.request_params
            .as_ref()
            .and_then(|params| params.get(request_key))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn new_or_fail(model_name: &str) -> ModelConfig {
        ModelConfig::new(model_name)
            .unwrap_or_else(|_| panic!("Failed to create model config for {}", model_name))
    }
}

#[derive(Clone)]
pub struct ModelConfigResolver {
    runtime: Arc<ProviderRuntime>,
}

impl ModelConfigResolver {
    pub fn new(runtime: Arc<ProviderRuntime>) -> Self {
        Self { runtime }
    }

    pub fn resolve(
        &self,
        provider_name: &str,
        model_name: &str,
    ) -> Result<ModelConfig, ModelConfigError> {
        self.apply(provider_name, ModelConfig::new(model_name)?)
    }

    pub fn resolve_with_context_key(
        &self,
        provider_name: &str,
        model_name: &str,
        context_key: Option<&str>,
    ) -> Result<ModelConfig, ModelConfigError> {
        self.apply_with_context_key(provider_name, ModelConfig::new(model_name)?, context_key)
    }

    pub fn apply(
        &self,
        provider_name: &str,
        config: ModelConfig,
    ) -> Result<ModelConfig, ModelConfigError> {
        self.apply_with_context_key(provider_name, config, None)
    }

    pub fn apply_with_context_key(
        &self,
        provider_name: &str,
        config: ModelConfig,
        context_key: Option<&str>,
    ) -> Result<ModelConfig, ModelConfigError> {
        self.apply_inner(provider_name, config, context_key, true)
    }

    fn apply_inner(
        &self,
        provider_name: &str,
        mut config: ModelConfig,
        context_key: Option<&str>,
        include_fast_model: bool,
    ) -> Result<ModelConfig, ModelConfigError> {
        let store = self.runtime.config.as_ref();

        if config.context_limit.is_none() {
            config.context_limit = read_context_limit(store, context_key)?;
        }
        if config.max_tokens.is_none() {
            config.max_tokens =
                read_optional_config(store, "GOOSE_MAX_TOKENS", ModelConfig::parse_max_tokens)?;
        }
        if config.temperature.is_none() {
            config.temperature =
                read_optional_config(store, "GOOSE_TEMPERATURE", ModelConfig::parse_temperature)?;
        }
        if !config.toolshim {
            config.toolshim =
                read_optional_config(store, "GOOSE_TOOLSHIM", ModelConfig::parse_toolshim)?
                    .unwrap_or(false);
        }
        if config.toolshim_model.is_none() {
            config.toolshim_model = read_optional_config(
                store,
                "GOOSE_TOOLSHIM_OLLAMA_MODEL",
                ModelConfig::parse_toolshim_model,
            )?;
        }

        if let Some(predefined) = find_predefined_model(store, &config.model_name)? {
            if config.context_limit.is_none() {
                config.context_limit = predefined.context_limit;
            }
            if let Some(params) = predefined.request_params {
                merge_request_params(&mut config.request_params, params);
            }
        }

        apply_thinking_effort(store, &mut config)?;
        config = config.with_canonical_limits(provider_name);

        if include_fast_model && config.fast_model_config.is_none() {
            if let Some(fast_model_name) = read_optional_string(store, "GOOSE_FAST_MODEL")? {
                let fast_config = self.apply_inner(
                    provider_name,
                    ModelConfig::new(&fast_model_name)?,
                    context_key,
                    false,
                )?;
                config.fast_model_config = Some(Box::new(fast_config));
            }
        }

        Ok(config)
    }
}

fn merge_request_params(
    target: &mut Option<HashMap<String, Value>>,
    params: HashMap<String, Value>,
) {
    let existing = target.get_or_insert_with(HashMap::new);
    for (key, value) in params {
        existing.entry(key).or_insert(value);
    }
}

fn merge_request_params_overriding(
    target: &mut Option<HashMap<String, Value>>,
    params: HashMap<String, Value>,
) {
    let existing = target.get_or_insert_with(HashMap::new);
    for (key, value) in params {
        existing.insert(key, value);
    }
}

fn apply_thinking_effort(
    store: &dyn ProviderConfigStore,
    config: &mut ModelConfig,
) -> Result<(), ModelConfigError> {
    if config
        .request_params
        .as_ref()
        .is_some_and(|params| params.contains_key("thinking_effort"))
    {
        return Ok(());
    }

    let effort = read_optional_string(store, "GOOSE_THINKING_EFFORT")?
        .and_then(|value| value.parse::<ThinkingEffort>().ok());
    let effort = match effort {
        Some(effort) => Some(effort),
        None => match legacy_thinking_effort(store) {
            Some(Ok(effort)) => Some(effort),
            Some(Err(error)) => return Err(error),
            None => None,
        },
    };

    if let Some(effort) = effort {
        config
            .request_params
            .get_or_insert_with(HashMap::new)
            .insert(
                "thinking_effort".to_string(),
                serde_json::json!(effort.to_string()),
            );
    }

    Ok(())
}

fn legacy_thinking_effort(
    store: &dyn ProviderConfigStore,
) -> Option<Result<ThinkingEffort, ModelConfigError>> {
    match read_optional_string(store, "CLAUDE_THINKING_TYPE") {
        Ok(Some(value)) => {
            if let Some(effort) = match value.to_lowercase().as_str() {
                "adaptive" | "enabled" => Some(ThinkingEffort::High),
                "disabled" => Some(ThinkingEffort::Off),
                _ => None,
            } {
                return Some(Ok(effort));
            }
        }
        Ok(None) => {}
        Err(e) => return Some(Err(e)),
    }

    match read_optional_config(store, "CLAUDE_THINKING_ENABLED", value_to_bool) {
        Ok(Some(enabled)) => {
            return Some(Ok(if enabled {
                ThinkingEffort::High
            } else {
                ThinkingEffort::Off
            }));
        }
        Ok(None) => {}
        Err(e) => return Some(Err(e)),
    }

    match read_optional_string(store, "GEMINI3_THINKING_LEVEL") {
        Ok(Some(value)) => ModelConfig::legacy_gemini3_thinking_effort(&value).map(Ok),
        Ok(None) => None,
        Err(e) => Some(Err(e)),
    }
}

fn read_context_limit(
    store: &dyn ProviderConfigStore,
    context_key: Option<&str>,
) -> Result<Option<usize>, ModelConfigError> {
    if let Some(key) = context_key {
        if let Some(limit) = read_optional_config(store, key, ModelConfig::parse_context_limit)? {
            return Ok(Some(limit));
        }
    }

    read_optional_config(
        store,
        "GOOSE_CONTEXT_LIMIT",
        ModelConfig::parse_context_limit,
    )
}

fn read_optional_config<T, F>(
    store: &dyn ProviderConfigStore,
    key: &str,
    parse: F,
) -> Result<Option<T>, ModelConfigError>
where
    F: FnOnce(Value, &str) -> Result<T, ModelConfigError>,
{
    match store.get_param_value(key) {
        Ok(value) => parse(value, key).map(Some),
        Err(ProviderConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(ConfigError::InvalidValue(
            key.to_string(),
            String::new(),
            e.to_string(),
        )),
    }
}

fn read_optional_string(
    store: &dyn ProviderConfigStore,
    key: &str,
) -> Result<Option<String>, ModelConfigError> {
    read_optional_config(store, key, value_to_string)
}

fn get_predefined_models(
    store: &dyn ProviderConfigStore,
) -> Result<Vec<PredefinedModel>, ModelConfigError> {
    let Some(value) = read_optional_raw(store, "GOOSE_PREDEFINED_MODELS")? else {
        return Ok(Vec::new());
    };

    match value {
        Value::String(json) => serde_json::from_str(&json).map_err(|e| {
            ConfigError::InvalidValue("GOOSE_PREDEFINED_MODELS".to_string(), json, e.to_string())
        }),
        other => serde_json::from_value(other).map_err(|e| {
            ConfigError::InvalidValue(
                "GOOSE_PREDEFINED_MODELS".to_string(),
                String::new(),
                e.to_string(),
            )
        }),
    }
}

fn find_predefined_model(
    store: &dyn ProviderConfigStore,
    model_name: &str,
) -> Result<Option<PredefinedModel>, ModelConfigError> {
    Ok(get_predefined_models(store)?
        .into_iter()
        .find(|model| model.name == model_name))
}

fn read_optional_raw(
    store: &dyn ProviderConfigStore,
    key: &str,
) -> Result<Option<Value>, ModelConfigError> {
    match store.get_param_value(key) {
        Ok(value) => Ok(Some(value)),
        Err(ProviderConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(ConfigError::InvalidValue(
            key.to_string(),
            String::new(),
            e.to_string(),
        )),
    }
}

fn value_to_usize(value: Value, key: &str) -> Result<usize, ModelConfigError> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| value as usize).ok_or_else(|| {
            ConfigError::InvalidValue(
                key.to_string(),
                number.to_string(),
                "must be a positive integer".to_string(),
            )
        }),
        Value::String(value) => value.parse::<usize>().map_err(|_| {
            ConfigError::InvalidValue(
                key.to_string(),
                value,
                "must be a positive integer".to_string(),
            )
        }),
        other => Err(ConfigError::InvalidValue(
            key.to_string(),
            other.to_string(),
            "must be a positive integer".to_string(),
        )),
    }
}

fn value_to_i32(value: Value, key: &str) -> Result<i32, ModelConfigError> {
    match value {
        Value::Number(number) => number.as_i64().map(|value| value as i32).ok_or_else(|| {
            ConfigError::InvalidValue(
                key.to_string(),
                number.to_string(),
                "must be a valid integer".to_string(),
            )
        }),
        Value::String(value) => value.parse::<i32>().map_err(|_| {
            ConfigError::InvalidValue(
                key.to_string(),
                value,
                "must be a valid integer".to_string(),
            )
        }),
        other => Err(ConfigError::InvalidValue(
            key.to_string(),
            other.to_string(),
            "must be a valid integer".to_string(),
        )),
    }
}

fn value_to_f32(value: Value, key: &str) -> Result<f32, ModelConfigError> {
    match value {
        Value::Number(number) => number.as_f64().map(|value| value as f32).ok_or_else(|| {
            ConfigError::InvalidValue(
                key.to_string(),
                number.to_string(),
                "must be a valid number".to_string(),
            )
        }),
        Value::String(value) => value.parse::<f32>().map_err(|_| {
            ConfigError::InvalidValue(key.to_string(), value, "must be a valid number".to_string())
        }),
        other => Err(ConfigError::InvalidValue(
            key.to_string(),
            other.to_string(),
            "must be a valid number".to_string(),
        )),
    }
}

fn value_to_bool(value: Value, key: &str) -> Result<bool, ModelConfigError> {
    match value {
        Value::Bool(value) => Ok(value),
        Value::Number(number) => match number.as_i64() {
            Some(1) => Ok(true),
            Some(0) => Ok(false),
            _ => Err(ConfigError::InvalidValue(
                key.to_string(),
                number.to_string(),
                "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
            )),
        },
        Value::String(value) => match value.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::InvalidValue(
                key.to_string(),
                value,
                "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
            )),
        },
        other => Err(ConfigError::InvalidValue(
            key.to_string(),
            other.to_string(),
            "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
        )),
    }
}

fn value_to_string(value: Value, key: &str) -> Result<String, ModelConfigError> {
    match value {
        Value::String(value) => Ok(value),
        other => serde_json::from_value::<String>(other.clone()).map_err(|e| {
            ConfigError::InvalidValue(key.to_string(), other.to_string(), e.to_string())
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfigStore;
    use std::sync::Mutex;

    #[derive(Default)]
    struct TestConfig {
        params: Mutex<HashMap<String, Value>>,
    }

    impl TestConfig {
        fn with_param(self, key: &str, value: impl Into<Value>) -> Self {
            self.params
                .lock()
                .unwrap()
                .insert(key.to_string(), value.into());
            self
        }
    }

    impl ProviderConfigStore for TestConfig {
        fn get_param_value(&self, key: &str) -> Result<Value, ProviderConfigError> {
            self.params
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| ProviderConfigError::NotFound(key.to_string()))
        }

        fn get_secret_value(&self, key: &str) -> Result<Value, ProviderConfigError> {
            Err(ProviderConfigError::NotFound(key.to_string()))
        }

        fn get_secret_group(
            &self,
            _primary: &str,
            _maybe_secret: &[&str],
        ) -> Result<HashMap<String, String>, ProviderConfigError> {
            Ok(HashMap::new())
        }

        fn set_param_value(&self, key: &str, value: Value) -> Result<(), ProviderConfigError> {
            self.params.lock().unwrap().insert(key.to_string(), value);
            Ok(())
        }

        fn set_secret_value(&self, _key: &str, _value: Value) -> Result<(), ProviderConfigError> {
            Ok(())
        }

        fn delete_secret(&self, _key: &str) -> Result<(), ProviderConfigError> {
            Ok(())
        }

        fn invalidate_secrets_cache(&self) {}
    }

    fn resolver(config: TestConfig) -> ModelConfigResolver {
        ModelConfigResolver::new(Arc::new(ProviderRuntime {
            config: Arc::new(config),
        }))
    }

    #[test]
    fn model_config_new_is_pure() {
        let _guard = env_lock::lock_env([
            ("GOOSE_MAX_TOKENS", Some("8192")),
            ("GOOSE_TEMPERATURE", Some("0.2")),
            ("GOOSE_CONTEXT_LIMIT", Some("65536")),
            ("GOOSE_TOOLSHIM", Some("true")),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", Some("shim")),
        ]);

        let config = ModelConfig::new("test-model").unwrap();
        assert_eq!(config.context_limit, None);
        assert_eq!(config.max_tokens, None);
        assert_eq!(config.temperature, None);
        assert!(!config.toolshim);
        assert_eq!(config.toolshim_model, None);
    }

    #[test]
    fn resolver_applies_injected_config() {
        let config = TestConfig::default()
            .with_param("GOOSE_CONTEXT_LIMIT", 65_536)
            .with_param("GOOSE_MAX_TOKENS", 4096)
            .with_param("GOOSE_TEMPERATURE", 0.2)
            .with_param("GOOSE_TOOLSHIM", true)
            .with_param("GOOSE_TOOLSHIM_OLLAMA_MODEL", "qwen3:0.6b")
            .with_param("GOOSE_THINKING_EFFORT", "high");

        let config = resolver(config).resolve("openai", "gpt-4o").unwrap();

        assert_eq!(config.context_limit, Some(65_536));
        assert_eq!(config.max_tokens, Some(4096));
        assert_eq!(config.temperature, Some(0.2));
        assert!(config.toolshim);
        assert_eq!(config.toolshim_model.as_deref(), Some("qwen3:0.6b"));
        assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
    }

    #[test]
    fn resolver_prefers_context_override_key() {
        let config = TestConfig::default()
            .with_param("GOOSE_CONTEXT_LIMIT", 65_536)
            .with_param("GOOSE_PLANNER_CONTEXT_LIMIT", 131_072);

        let config = resolver(config)
            .resolve_with_context_key("openai", "gpt-4o", Some("GOOSE_PLANNER_CONTEXT_LIMIT"))
            .unwrap();

        assert_eq!(config.context_limit, Some(131_072));
    }

    #[test]
    fn resolver_applies_predefined_models_from_injected_config() {
        let predefined = serde_json::json!([
            {
                "name": "custom-model",
                "context_limit": 42_000,
                "request_params": {"thinking_effort": "low", "custom": true}
            }
        ]);
        let config = TestConfig::default().with_param("GOOSE_PREDEFINED_MODELS", predefined);

        let config = resolver(config).resolve("openai", "custom-model").unwrap();

        assert_eq!(config.context_limit, Some(42_000));
        assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Low));
        assert_eq!(
            config
                .request_params
                .as_ref()
                .and_then(|params| params.get("custom")),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn resolver_sets_fast_model_from_injected_config() {
        let config = TestConfig::default().with_param("GOOSE_FAST_MODEL", "gpt-4o-mini");

        let config = resolver(config).resolve("openai", "gpt-4o").unwrap();

        assert_eq!(
            config
                .fast_model_config
                .as_ref()
                .map(|config| config.model_name.as_str()),
            Some("gpt-4o-mini")
        );
    }

    #[test]
    fn resolver_does_not_override_explicit_values() {
        let config = TestConfig::default()
            .with_param("GOOSE_CONTEXT_LIMIT", 65_536)
            .with_param("GOOSE_MAX_TOKENS", 4096)
            .with_param("GOOSE_TEMPERATURE", 0.2)
            .with_param("GOOSE_TOOLSHIM", true);

        let model = ModelConfig::new("gpt-4o")
            .unwrap()
            .with_context_limit(Some(32_000))
            .with_max_tokens(Some(1024))
            .with_temperature(Some(0.8))
            .with_toolshim(false);

        let config = resolver(config).apply("openai", model).unwrap();

        assert_eq!(config.context_limit, Some(32_000));
        assert_eq!(config.max_tokens, Some(1024));
        assert_eq!(config.temperature, Some(0.8));
        assert!(config.toolshim);
    }

    #[test]
    fn invalid_max_tokens_returns_error() {
        let config = TestConfig::default().with_param("GOOSE_MAX_TOKENS", 0);
        let result = resolver(config).resolve("openai", "gpt-4o");
        assert!(matches!(result, Err(ConfigError::InvalidRange(..))));
    }

    #[test]
    fn request_params_are_used_for_thinking_effort_without_config_reads() {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", Some("high"))]);
        let mut params = HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("low"));
        let config = ModelConfig {
            model_name: "test".to_string(),
            request_params: Some(params),
            ..Default::default()
        };

        assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Low));
    }

    #[test]
    fn effort_suffix_stripped_from_model_name() {
        let config = ModelConfig::new("o3-mini-high").unwrap();
        assert_eq!(config.model_name, "o3-mini");
        assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
    }

    #[test]
    fn with_canonical_limits_sets_limits_from_canonical_model() {
        let config = ModelConfig::new_or_fail("gpt-4o").with_canonical_limits("openai");

        assert_eq!(config.context_limit, Some(128_000));
        assert_eq!(config.max_tokens, Some(16_384));
        assert_eq!(config.reasoning, Some(false));
    }

    #[test]
    fn with_canonical_limits_does_not_override_existing_context_limit() {
        let mut config = ModelConfig::new_or_fail("gpt-4o");
        config.context_limit = Some(64_000);
        let config = config.with_canonical_limits("openai");

        assert_eq!(config.context_limit, Some(64_000));
    }

    #[test]
    fn with_canonical_limits_skips_output_limit_when_it_equals_context_limit() {
        let config =
            ModelConfig::new_or_fail("moonshotai/kimi-k2.6").with_canonical_limits("nvidia");

        assert_eq!(config.context_limit, Some(262_144));
        assert_eq!(config.max_tokens, None);
        assert_eq!(config.max_output_tokens(), 4_096);
    }

    #[test]
    fn with_canonical_limits_resolves_after_stripping_reasoning_effort_suffix() {
        let config =
            ModelConfig::new_or_fail("databricks-gpt-5.4-high").with_canonical_limits("databricks");
        assert_eq!(config.context_limit, Some(1_050_000));

        let config = ModelConfig::new_or_fail("gpt-5.4-xhigh").with_canonical_limits("openai");
        assert_eq!(config.context_limit, Some(1_050_000));
    }

    #[test]
    fn parse_aliases() {
        assert_eq!("off".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Off));
        assert_eq!(
            "disabled".parse::<ThinkingEffort>(),
            Ok(ThinkingEffort::Off)
        );
        assert_eq!("med".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Medium));
        assert_eq!("max".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
        assert_eq!("xhigh".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
        assert!("invalid".parse::<ThinkingEffort>().is_err());
    }

    #[test]
    fn is_openai_reasoning_model_detects_expected_models() {
        assert!(ModelConfig::new_or_fail("o1").is_openai_reasoning_model());
        assert!(ModelConfig::new_or_fail("o3-mini").is_openai_reasoning_model());
        assert!(ModelConfig::new_or_fail("gpt-5").is_openai_reasoning_model());
        assert!(ModelConfig::new_or_fail("goose-gpt-5").is_openai_reasoning_model());
        assert!(ModelConfig::new_or_fail("databricks-gpt-5").is_openai_reasoning_model());
        assert!(!ModelConfig::new_or_fail("claude-sonnet-4").is_openai_reasoning_model());
        assert!(!ModelConfig::new_or_fail("gpt-4o").is_openai_reasoning_model());
    }

    #[test]
    fn is_reasoning_model_uses_explicit_metadata_first() {
        let mut config = ModelConfig::new_or_fail("provider-alias");
        config.reasoning = Some(true);
        assert!(config.is_reasoning_model());

        let mut config = ModelConfig::new_or_fail("claude-sonnet-4");
        config.reasoning = Some(false);
        assert!(!config.is_reasoning_model());
    }
}
