use crate::config_constants::*;
pub fn toml_value_to_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        toml::Value::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

fn llm_table(table: &toml::value::Table) -> Option<&toml::value::Table> {
    table.get(TOML_LLM_TABLE).and_then(toml::Value::as_table)
}

fn non_empty_toml_string(table: &toml::value::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml_value_to_string)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn selected_profile_table(table: &toml::value::Table) -> Option<&toml::value::Table> {
    let profile_name = llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_PROFILE))
        .or_else(|| non_empty_toml_string(table, TOML_LLM_PROFILE))?;
    table
        .get(TOML_PROFILES_TABLE)
        .and_then(toml::Value::as_table)
        .and_then(|profiles| profiles.get(profile_name.as_str()))
        .and_then(toml::Value::as_table)
}

fn selected_model_provider_name(table: &toml::value::Table) -> Option<String> {
    if let Some(provider) =
        llm_table(table).and_then(|llm| non_empty_toml_string(llm, TOML_LLM_MODEL_PROVIDER))
    {
        return Some(provider);
    }
    if let Some(provider) = selected_profile_table(table)
        .and_then(|profile| non_empty_toml_string(profile, TOML_LLM_MODEL_PROVIDER))
    {
        return Some(provider);
    }
    if let Some(provider) = non_empty_toml_string(table, TOML_LLM_MODEL_PROVIDER) {
        return Some(provider);
    }
    let providers = table
        .get(TOML_MODEL_PROVIDERS_TABLE)
        .and_then(toml::Value::as_table)?;
    if providers.len() == 1 {
        return providers.keys().next().cloned();
    }
    None
}

fn selected_model_provider_table(table: &toml::value::Table) -> Option<&toml::value::Table> {
    let provider_name = selected_model_provider_name(table)?;
    table
        .get(TOML_MODEL_PROVIDERS_TABLE)
        .and_then(toml::Value::as_table)
        .and_then(|providers| providers.get(provider_name.as_str()))
        .and_then(toml::Value::as_table)
}

fn resolve_model_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_MODEL))
        .or_else(|| {
            selected_profile_table(table)
                .and_then(|profile| non_empty_toml_string(profile, TOML_LLM_MODEL))
        })
        .or_else(|| non_empty_toml_string(table, TOML_LLM_MODEL))
}

fn resolve_base_url_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_BASE_URL))
        .or_else(|| {
            selected_model_provider_table(table)
                .and_then(|provider| non_empty_toml_string(provider, TOML_LLM_BASE_URL))
        })
}

fn resolve_api_key_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_API_KEY))
        .or_else(|| {
            selected_model_provider_table(table).and_then(|provider| {
                non_empty_toml_string(provider, TOML_MODEL_PROVIDER_AUTH_TOKEN)
            })
        })
        .or_else(|| {
            selected_model_provider_table(table)
                .and_then(|provider| non_empty_toml_string(provider, TOML_LLM_API_KEY))
        })
}

fn resolve_llm_key_from_config(table: &toml::value::Table, key: &str) -> Option<String> {
    llm_table(table).and_then(|llm| non_empty_toml_string(llm, key))
}

fn resolve_agent_goal_override_from_config(
    table: &toml::value::Table,
    key: &str,
) -> Option<String> {
    let (normalized_agent, field) =
        if let Some(suffix) = key.strip_prefix(ENV_LLM_SHORT_TERM_GOAL_AGENT_PREFIX) {
            (suffix, TOML_LLM_SHORT_TERM_GOAL)
        } else if let Some(suffix) = key.strip_prefix(ENV_LLM_LONG_TERM_GOAL_AGENT_PREFIX) {
            (suffix, TOML_LLM_LONG_TERM_GOAL)
        } else {
            return None;
        };
    llm_table(table)
        .and_then(|llm| llm.get(TOML_LLM_AGENT_OVERRIDES_TABLE))
        .and_then(toml::Value::as_table)
        .and_then(|overrides| overrides.get(normalized_agent))
        .and_then(toml::Value::as_table)
        .and_then(|override_entry| non_empty_toml_string(override_entry, field))
}

pub fn config_value_for_env_key(table: &toml::value::Table, key: &str) -> Option<String> {
    match key {
        ENV_LLM_MODEL => resolve_model_from_config(table),
        ENV_LLM_BASE_URL => resolve_base_url_from_config(table),
        ENV_LLM_API_KEY => resolve_api_key_from_config(table),
        ENV_LLM_TIMEOUT_MS => resolve_llm_key_from_config(table, TOML_LLM_TIMEOUT_MS),
        ENV_LLM_SYSTEM_PROMPT => resolve_llm_key_from_config(table, TOML_LLM_SYSTEM_PROMPT),
        ENV_LLM_SHORT_TERM_GOAL => resolve_llm_key_from_config(table, TOML_LLM_SHORT_TERM_GOAL),
        ENV_LLM_LONG_TERM_GOAL => resolve_llm_key_from_config(table, TOML_LLM_LONG_TERM_GOAL),
        ENV_LLM_MAX_MODULE_CALLS => resolve_llm_key_from_config(table, TOML_LLM_MAX_MODULE_CALLS),
        ENV_LLM_MAX_DECISION_STEPS => {
            resolve_llm_key_from_config(table, TOML_LLM_MAX_DECISION_STEPS)
        }
        ENV_LLM_MAX_REPAIR_ROUNDS => resolve_llm_key_from_config(table, TOML_LLM_MAX_REPAIR_ROUNDS),
        ENV_LLM_PROMPT_MAX_HISTORY_ITEMS => {
            resolve_llm_key_from_config(table, TOML_LLM_PROMPT_MAX_HISTORY_ITEMS)
        }
        ENV_LLM_PROMPT_PROFILE => resolve_llm_key_from_config(table, TOML_LLM_PROMPT_PROFILE),
        ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION => {
            resolve_llm_key_from_config(table, TOML_LLM_FORCE_REPLAN_AFTER_SAME_ACTION)
        }
        ENV_LLM_HARVEST_MAX_AMOUNT_CAP => {
            resolve_llm_key_from_config(table, TOML_LLM_HARVEST_MAX_AMOUNT_CAP)
        }
        ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS => {
            resolve_llm_key_from_config(table, TOML_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS)
        }
        ENV_LLM_DEBUG_MODE => resolve_llm_key_from_config(table, TOML_LLM_DEBUG_MODE),
        _ => resolve_agent_goal_override_from_config(table, key),
    }
}

/// Inference configuration only. Canonical task goals and execution authority
/// are provided by the host for each request, never inferred from this file.
#[derive(Clone)]
pub struct ModelProviderConfig {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub timeout_ms: u64,
    pub system_prompt: String,
    pub short_term_goal: Option<String>,
    pub long_term_goal: Option<String>,
    pub max_module_calls: usize,
    pub max_decision_steps: usize,
    pub max_repair_rounds: usize,
}
impl std::fmt::Debug for ModelProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelProviderConfig")
            .field("model", &self.model)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}
impl crate::CompletionClientConfig for ModelProviderConfig {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}
impl ModelProviderConfig {
    pub fn from_default_sources_for_agent(agent: &str) -> Result<Self, String> {
        let path = std::path::Path::new("config.toml");
        if path.exists() {
            Self::from_config_file_for_agent(path, agent)
        } else {
            Self::from_values_for_agent(|key| std::env::var(key).ok(), agent)
        }
    }
    pub fn from_config_file_for_agent(path: &std::path::Path, agent: &str) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|_| "model provider config unavailable")?;
        let value: toml::Value =
            toml::from_str(&text).map_err(|_| "invalid model provider config")?;
        let table = value
            .as_table()
            .ok_or("model provider config must be a table")?;
        Self::from_values_for_agent(
            |key| config_value_for_env_key(table, key).or_else(|| std::env::var(key).ok()),
            agent,
        )
    }
    pub fn from_values_for_agent(
        mut get: impl FnMut(&str) -> Option<String>,
        agent: &str,
    ) -> Result<Self, String> {
        let model = get(ENV_LLM_MODEL)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_LLM_MODEL.into())
            .trim()
            .to_owned();
        let mut required = |key| {
            get(key)
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("missing model provider setting {key}"))
        };
        let base_url = required(ENV_LLM_BASE_URL)?;
        let api_key = required(ENV_LLM_API_KEY)?;
        let timeout_ms = get(ENV_LLM_TIMEOUT_MS)
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| "invalid model provider timeout".to_owned())
            })
            .transpose()?
            .unwrap_or(180_000);
        let system_prompt = get(ENV_LLM_SYSTEM_PROMPT)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_default();
        let short_term_goal = goal_value(&mut get, ENV_LLM_SHORT_TERM_GOAL, agent);
        let long_term_goal = goal_value(&mut get, ENV_LLM_LONG_TERM_GOAL, agent);
        let mut positive = |key, default| {
            get(key)
                .map(|v| {
                    v.parse::<usize>()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| format!("invalid model provider budget {key}"))
                })
                .unwrap_or(Ok(default))
        };
        let max_module_calls = positive(ENV_LLM_MAX_MODULE_CALLS, 3)?;
        let max_decision_steps = positive(ENV_LLM_MAX_DECISION_STEPS, 4)?;
        let max_repair_rounds = positive(ENV_LLM_MAX_REPAIR_ROUNDS, 1)?;
        Ok(Self {
            model,
            base_url,
            api_key,
            timeout_ms,
            system_prompt,
            short_term_goal,
            long_term_goal,
            max_module_calls,
            max_decision_steps,
            max_repair_rounds,
        })
    }
}

pub fn goal_value<F>(mut getter: &mut F, key: &str, agent_id: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    agent_scoped_goal_key(key, agent_id)
        .as_deref()
        .and_then(&mut getter)
        .or_else(|| getter(key))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn agent_scoped_goal_key(key: &str, agent_id: &str) -> Option<String> {
    let normalized = normalize_agent_id_for_env(agent_id)?;
    Some(format!("{key}_{normalized}"))
}

fn normalize_agent_id_for_env(agent_id: &str) -> Option<String> {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = String::with_capacity(trimmed.len());
    let mut last_is_underscore = false;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_uppercase());
            last_is_underscore = false;
        } else if !last_is_underscore {
            normalized.push('_');
            last_is_underscore = true;
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}
