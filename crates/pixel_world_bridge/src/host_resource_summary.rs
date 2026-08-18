use serde_json::Value;

pub(super) fn resource_summary(resources: &Value) -> String {
    let Some(resources) = resources.as_object() else {
        return "-".to_string();
    };
    let entries: Vec<String> = resources
        .iter()
        .map(|(key, value)| {
            if value.is_object() {
                format!("{key}:{}", value)
            } else if let Some(text) = value.as_str() {
                format!("{key}:{text}")
            } else {
                format!("{key}:{value}")
            }
        })
        .collect();
    if entries.is_empty() {
        "-".to_string()
    } else {
        entries.join(" · ")
    }
}

pub(super) fn count_resource_entries(summary: &str) -> usize {
    if summary.is_empty() || summary == "-" {
        return 0;
    }
    summary
        .split(" · ")
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .count()
}
