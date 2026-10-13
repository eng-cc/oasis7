use super::dto::*;
use crate::{Error, Result};
use serde_json::{Value, json};
fn string(value: &Value, key: &str) -> std::result::Result<String, String> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing or invalid {key}"))
}
pub fn server_request(
    frame: &Value,
    snapshot: &RuntimeSnapshot,
) -> std::result::Result<RuntimeRequest, String> {
    if serde_json::to_vec(frame)
        .map_err(|_| "request encoding")?
        .len()
        > 16384
    {
        return Err("critical request metadata exceeds 16KiB bound".into());
    }
    let params = &frame["params"];
    let method = string(frame, "method")?;
    if ![
        "item/commandExecution/requestApproval",
        "item/fileChange/requestApproval",
        "item/tool/requestUserInput",
    ]
    .contains(&method.as_str())
    {
        return Err("request method is not supported by this connector".into());
    }
    let thread_id = string(params, "threadId")?;
    let turn_id = string(params, "turnId")?;
    let _ = string(params, "itemId")?;
    let instance = snapshot
        .instance_id
        .clone()
        .ok_or("runtime identity missing")?;
    let session = snapshot
        .sessions
        .iter()
        .find(|s| s.thread_id == thread_id && s.instance_id == instance && s.owned)
        .ok_or("request belongs to unowned session")?;
    if session.turn_id.as_ref().is_some_and(|id| id != &turn_id) {
        return Err("request has a stale turn identity".into());
    }
    let mut questions = Vec::new();
    let mut decisions = Vec::new();
    if method == "item/tool/requestUserInput" {
        let raw = params["questions"]
            .as_array()
            .filter(|q| !q.is_empty() && q.len() <= 8)
            .ok_or("invalid questions")?;
        for q in raw {
            let id = string(q, "id")?;
            if questions.iter().any(|old: &RuntimeQuestion| old.id == id) {
                return Err("duplicate question identity".into());
            }
            let options = match q.get("options") {
                Some(Value::Array(list)) => {
                    if list.len() > 20 {
                        return Err("too many question options".into());
                    }
                    list.iter()
                        .map(|v| string(v, "label"))
                        .collect::<std::result::Result<Vec<_>, _>>()?
                }
                None | Some(Value::Null) => Vec::new(),
                _ => return Err("invalid question options".into()),
            };
            questions.push(RuntimeQuestion {
                id,
                header: string(q, "header")?,
                question: string(q, "question")?,
                options,
                is_secret: q["isSecret"].as_bool().unwrap_or(false),
            });
        }
    } else {
        if !params["startedAtMs"].is_number() {
            return Err("missing approval start identity".into());
        }
        decisions = vec![
            "accept".into(),
            "acceptForSession".into(),
            "decline".into(),
            "cancel".into(),
        ];
    }
    let mut scope_fields = Vec::new();
    for field in [
        "itemId",
        "approvalId",
        "command",
        "cwd",
        "grantRoot",
        "reason",
        "commandActions",
        "networkApprovalContext",
        "proposedExecpolicyAmendment",
        "proposedNetworkPolicyAmendments",
    ] {
        if let Some(value) = params.get(field).filter(|v| !v.is_null()) {
            scope_fields.push(format!("{field}: {value}"));
        }
    }
    let summary = scope_fields.join("\n");
    if summary.len() > 16384 {
        return Err(
            "approval scope exceeds display bound; refusing incomplete permission summary".into(),
        );
    }
    Ok(RuntimeRequest {
        request_id: frame["id"].to_string(),
        instance_id: instance,
        thread_id,
        turn_id,
        method,
        state: "pending".into(),
        summary,
        questions,
        allowed_decisions: decisions,
    })
}
pub fn answer(request: &RuntimeRequest, reply: &RequestAnswer) -> Result<Value> {
    if request.request_id != reply.request_id
        || request.instance_id != reply.target.instance_id
        || request.thread_id != reply.target.thread_id
        || request.turn_id != reply.target.turn_id
        || request.state != "pending"
    {
        return Err(Error::Invalid(
            "request identity is stale, answered, or belongs to another instance".into(),
        ));
    }
    if request.method == "item/tool/requestUserInput" {
        if reply.decision.is_some() || reply.answers.len() != request.questions.len() {
            return Err(Error::Invalid(
                "reply must answer exactly the requested questions".into(),
            ));
        }
        let mut answers = serde_json::Map::new();
        for question in &request.questions {
            let values = reply
                .answers
                .get(&question.id)
                .ok_or_else(|| Error::Invalid("missing question answer".into()))?;
            if values.is_empty() || values.len() > 20 || values.iter().any(|v| v.len() > 8192) {
                return Err(Error::Invalid("answer exceeds supported bounds".into()));
            }
            answers.insert(question.id.clone(), json!({"answers":values}));
        }
        Ok(json!({"answers":answers}))
    } else {
        let decision = reply
            .decision
            .as_deref()
            .ok_or_else(|| Error::Invalid("approval decision required".into()))?;
        if !reply.answers.is_empty() || !request.allowed_decisions.iter().any(|s| s == decision) {
            return Err(Error::Invalid(
                "approval reply is not a supported decision".into(),
            ));
        }
        Ok(json!({"decision":decision}))
    }
}
pub fn notification(method: &str, params: &Value, snapshot: &mut RuntimeSnapshot) {
    let Some(thread) = params["threadId"].as_str() else {
        return;
    };
    let Some(session) = snapshot.sessions.iter_mut().find(|s| {
        s.thread_id == thread && s.owned && Some(&s.instance_id) == snapshot.instance_id.as_ref()
    }) else {
        return;
    };
    match method {
        "turn/started" => {
            if let Some(id) = params["turn"]["id"].as_str() {
                session.turn_id = Some(id.into());
                session.state = "active".into();
                session.background_state = "unknown".into();
            }
        }
        "turn/completed" => {
            if params["turn"]["id"].as_str() == session.turn_id.as_deref() {
                session.state = match params["turn"]["status"].as_str() {
                    Some("failed") => "failed",
                    Some("interrupted") => "interrupted",
                    Some("completed") => "completed",
                    _ => "unknown",
                }
                .into();
                session.background_state = "unknown".into();
                if session.state == "failed" {
                    let info = &params["turn"]["error"]["codexErrorInfo"];
                    let known = [
                        "contextWindowExceeded",
                        "usageLimitExceeded",
                        "serverOverloaded",
                        "cyberPolicy",
                        "internalServerError",
                        "unauthorized",
                        "badRequest",
                        "threadRollbackFailed",
                        "sandboxError",
                        "other",
                        "httpConnectionFailed",
                        "responseStreamConnectionFailed",
                        "responseStreamDisconnected",
                        "responseTooManyFailedAttempts",
                        "activeTurnNotSteerable",
                    ];
                    let code = info
                        .as_str()
                        .or_else(|| {
                            info.as_object()
                                .and_then(|v| v.keys().next().map(String::as_str))
                        })
                        .filter(|v| known.contains(v))
                        .unwrap_or("unknown");
                    snapshot.error = Some(format!("Source turn failed: {code}"));
                }
            }
        }
        "thread/status/changed" => match params["status"]["type"].as_str() {
            Some("active") => {
                session.state = if params["status"]["activeFlags"]
                    .as_array()
                    .is_some_and(|flags| !flags.is_empty())
                {
                    "waiting"
                } else {
                    "active"
                }
                .into();
            }
            Some("idle") => {
                if ["active", "waiting", "unknown"].contains(&session.state.as_str()) {
                    session.state = "idle".into();
                    session.background_state = "unknown".into();
                }
            }
            Some("systemError" | "notLoaded") => {
                session.state = "unknown".into();
                session.background_state = "unknown".into();
            }
            _ => {}
        },
        "serverRequest/resolved" => {
            let id = params["requestId"].to_string();
            for request in &mut snapshot.requests {
                if request.request_id == id
                    && request.thread_id == thread
                    && request.instance_id == session.instance_id
                {
                    request.state = "resolved".into();
                }
            }
        }
        "item/started" if params["item"]["type"].as_str() == Some("commandExecution") => {
            session.background_state = "active".into();
        }
        _ => {}
    }
}

pub fn validate_reply(
    method: &str,
    value: &Value,
    thread: Option<&str>,
) -> std::result::Result<(), String> {
    match method {
        "thread/start" | "thread/resume" => {
            let id = value["thread"]["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("thread result lacks identity")?;
            if thread.is_some_and(|expected| expected != id) {
                return Err("response returned another thread identity".into());
            }
            if !value["model"].is_string()
                || !value["cwd"].is_string()
                || !value["thread"]["status"]["type"].is_string()
            {
                return Err("thread result lacks required source context".into());
            }
        }
        "turn/start" => {
            let _ = value["turn"]["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("turn result lacks identity")?;
            if !["inProgress", "completed", "failed", "interrupted"]
                .contains(&value["turn"]["status"].as_str().unwrap_or(""))
            {
                return Err("turn result lacks source status".into());
            }
        }
        "turn/steer" => {
            if !value["turnId"].is_string() {
                return Err("steer receipt lacks turn identity".into());
            }
        }
        "turn/interrupt" => {
            if !value.is_object() {
                return Err("interrupt receipt malformed".into());
            }
        }
        _ => return Err("unsupported response method".into()),
    }
    Ok(())
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    #[test]
    fn approval_reason_never_hides_command_or_permission_scope() {
        let project = crate::domain::Project {
            id: "p".into(),
            path: "/tmp".into(),
            git_common_dir: "/tmp/.git".into(),
            branch: None,
            head: "h".into(),
            environment: "local".into(),
        };
        let mut snapshot = super::super::empty(&project);
        snapshot.instance_id = Some("i".into());
        snapshot.sessions.push(RuntimeSession {
            purpose: "purpose".into(),
            scope: "scope".into(),
            model: None,
            basis_revision: "h".into(),
            thread_id: "t".into(),
            instance_id: "i".into(),
            turn_id: Some("u".into()),
            state: "active".into(),
            owned: true,
            background_state: "unknown".into(),
            restorable_history: false,
            private_sources: vec![],
        });
        let frame = json!({"id":1,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"u","itemId":"item","startedAtMs":1,"reason":"Need a test","command":"actual command","cwd":"/scope"}});
        let request = server_request(&frame, &snapshot).unwrap();
        for field in ["actual command", "Need a test", "/scope", "itemId"] {
            assert!(request.summary.contains(field));
        }
        let frame = json!({"id":2,"method":"item/fileChange/requestApproval","params":{"threadId":"t","turnId":"u","itemId":"files","startedAtMs":1,"reason":"write","grantRoot":"/expanded"}});
        assert!(
            server_request(&frame, &snapshot)
                .unwrap()
                .summary
                .contains("/expanded")
        );
    }
    #[test]
    fn malformed_or_foreign_receipts_never_confirm_effects() {
        assert!(validate_reply("thread/start", &json!({}), None).is_err());
        let good =
            json!({"thread":{"id":"owned","status":{"type":"idle"}},"model":"m","cwd":"/tmp"});
        assert!(validate_reply("thread/resume", &good, Some("other")).is_err());
        assert!(validate_reply("thread/resume", &good, Some("owned")).is_ok());
        assert!(
            validate_reply(
                "turn/start",
                &json!({"turn":{"id":"t","status":"invented"}}),
                None
            )
            .is_err()
        );
        assert!(validate_reply("arbitrary/method", &json!({}), None).is_err());
    }
}
