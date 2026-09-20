use super::LlmConfigError;

pub(super) fn goal_value<F>(mut getter: &mut F, key: &str, agent_id: &str) -> Option<String>
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

pub(super) fn toml_value_to_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        toml::Value::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(super) fn required_env<F>(getter: &mut F, key: &'static str) -> Result<String, LlmConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    let value = getter(key).ok_or(LlmConfigError::MissingEnv { key })?;
    if value.trim().is_empty() {
        return Err(LlmConfigError::EmptyEnv { key });
    }
    Ok(value)
}

pub(super) fn parse_positive_usize<F>(
    getter: &mut F,
    key: &'static str,
    default: usize,
    error: fn(String) -> LlmConfigError,
) -> Result<usize, LlmConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    match getter(key) {
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or_else(|| error(value)),
        None => Ok(default),
    }
}

pub(super) fn parse_positive_i64<F>(
    getter: &mut F,
    key: &'static str,
    default: i64,
    error: fn(String) -> LlmConfigError,
) -> Result<i64, LlmConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    match getter(key) {
        Some(value) => value
            .parse::<i64>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or_else(|| error(value)),
        None => Ok(default),
    }
}

pub(super) fn parse_non_negative_usize<F>(
    getter: &mut F,
    key: &'static str,
    default: usize,
    error: fn(String) -> LlmConfigError,
) -> Result<usize, LlmConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    match getter(key) {
        Some(value) => value.parse::<usize>().map_err(|_| error(value.clone())),
        None => Ok(default),
    }
}
