//! Both executables enter here before any signer installation or custody access.
use crate::{
    cli_input::{Arguments, invalid},
    error::{ContextError, Result},
    model::*,
    store::Store,
};
use serde_json::{Value, json};
use std::{
    io::{IsTerminal, Read, Write},
    path::PathBuf,
};

pub fn run(args: &[String], prefix: &str) -> i32 {
    let json_mode = args.iter().any(|arg| arg == "--json");
    let parsed = Arguments::parse(args);
    let operation = parsed
        .as_ref()
        .ok()
        .and_then(|a| a.positional.first())
        .filter(|s| {
            [
                "info", "init", "doctor", "status", "list", "search", "get", "put", "edit",
                "archive", "verify", "history", "restore", "snapshot",
            ]
            .contains(&s.as_str())
        })
        .cloned()
        .unwrap_or_else(|| "unknown".into());
    match parsed.and_then(|args| execute(&args, prefix)) {
        Ok(Output::Raw(bytes)) => {
            if std::io::stdout().write_all(&bytes).is_err() {
                11
            } else {
                0
            }
        }
        Ok(Output::Data {
            operation,
            store_id,
            data,
            warnings,
        }) => {
            let envelope = json!({"schema_version":"oasis7.context_result.v1", "status":"ok", "operation":operation, "store_id":store_id, "data":data, "warnings":warnings, "error":null});
            match print(envelope, json_mode) {
                Ok(()) => 0,
                Err(error) => print_error(&operation, error, json_mode),
            }
        }
        Err(error) => print_error(&operation, error, json_mode),
    }
}
fn print_error(operation: &str, error: ContextError, json_mode: bool) -> i32 {
    let code = error.exit_code();
    if json_mode {
        let envelope = json!({"schema_version":"oasis7.context_result.v1", "status":"error", "operation":operation, "store_id":null, "data":null, "warnings":[], "error":error});
        let _ = print(envelope, true);
    } else {
        eprintln!("{}: {}", error.code, error.message);
    }
    code
}
enum Output {
    Raw(Vec<u8>),
    Data {
        operation: String,
        store_id: Option<String>,
        data: Value,
        warnings: Vec<Value>,
    },
}
fn output(operation: &str, store_id: Option<String>, data: Value) -> Output {
    Output::Data {
        operation: operation.into(),
        store_id,
        data,
        warnings: vec![],
    }
}
fn print(envelope: Value, json_mode: bool) -> Result<()> {
    let bytes =
        serde_json::to_vec(&envelope).map_err(|_| invalid("result serialization failed"))?;
    let operation = envelope["operation"].as_str().unwrap_or("");
    let limit = if operation == "get" {
        8 * 1024 * 1024
    } else {
        2 * 1024 * 1024
    };
    if bytes.len() > limit {
        return Err(ContextError::new(
            "LIMIT_EXCEEDED",
            "result exceeds output limit",
        ));
    }
    let mut out = std::io::stdout().lock();
    if json_mode {
        out.write_all(&bytes)?;
        out.write_all(b"\n")?;
    } else {
        let data = &envelope["data"];
        if let Some(usage) = data.get("usage").and_then(Value::as_str) {
            writeln!(out, "{}", escape(usage))?;
        } else if let Some(raw) = data.get("raw_text").and_then(Value::as_str) {
            writeln!(
                out,
                "{} ({})",
                escape(data["id"].as_str().unwrap_or("")),
                data["revision"].as_str().unwrap_or("")
            )?;
            writeln!(out, "{}", escape(raw))?;
        } else {
            writeln!(
                out,
                "{}",
                escape(
                    &serde_json::to_string_pretty(data)
                        .map_err(|_| invalid("result serialization failed"))?
                )
            )?;
        }
    }
    Ok(())
}
fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && !['\n', '\t'].contains(&c) {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn serialize<T: serde::Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| invalid("result serialization failed"))
}
fn condition(args: &Arguments) -> Result<WriteCondition> {
    match (args.flag("if-absent"), args.value("expected-revision")) {
        (true, None) => Ok(WriteCondition::Absent),
        (false, Some(revision)) => Ok(WriteCondition::Revision(revision.into())),
        _ => Err(invalid("exactly one write precondition is required")),
    }
}
fn candidate(file: &str) -> Result<Vec<u8>> {
    let mut reader: Box<dyn Read> = if file == "-" {
        Box::new(std::io::stdin())
    } else {
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(file)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(ContextError::new(
                "UNSAFE_PATH",
                "candidate must be an ordinary file",
            ));
        }
        if metadata.len() > MAX_ENTRY_BYTES as u64 {
            return Err(ContextError::new(
                "LIMIT_EXCEEDED",
                "candidate exceeds entry limit",
            ));
        }
        Box::new(file)
    };
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take((MAX_ENTRY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(ContextError::new(
            "LIMIT_EXCEEDED",
            "candidate exceeds entry limit",
        ));
    }
    Ok(bytes)
}
fn absolute(value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(invalid("an absolute path is required"));
    }
    Ok(path)
}
fn git_error(error: crate::git::GitError) -> ContextError {
    ContextError::new(error.code, error.message)
}

fn execute(args: &Arguments, prefix: &str) -> Result<Output> {
    if args.flag("help") || args.positional.is_empty() {
        return Ok(output(
            "help",
            None,
            json!({"usage":format!("{prefix} [--root ABSOLUTE] [--json] COMMAND\nCommands: info --json, init, doctor, status, list, search QUERY, get ID, put ID --file FILE, edit ID, archive ID, verify ID, history ID, restore ID --from-commit OID, snapshot export|inspect|restore\nWrites require --if-absent or --expected-revision. init requires --root and --project. Snapshot passphrases: hidden TTY or --passphrase-stdin.\nUse the private-context design/manual for command options.")}),
        ));
    }
    let command = args.positional[0].as_str();
    if ![
        "info", "init", "doctor", "status", "list", "search", "get", "put", "edit", "archive",
        "verify", "history", "restore", "snapshot",
    ]
    .contains(&command)
    {
        return Err(invalid("unknown context command"));
    }
    if command == "info" {
        args.arity(1)?;
        args.allow(&[])?;
        return Ok(Output::Raw(
            serde_json::to_vec(&crate::application::info())
                .map_err(|_| invalid("result serialization failed"))?,
        ));
    }
    if command == "snapshot" {
        return snapshot(args);
    }
    let mut store;
    if command == "init" {
        args.arity(1)?;
        args.allow(&["project", "name"])?;
        let root = absolute(args.required("root")?)?;
        let project = args.required("project")?;
        store = Store::init(&root, project, args.value("name").unwrap_or(project))?;
        return Ok(output(
            "init",
            Some(store.config.store_id.clone()),
            serialize(&store.config)?,
        ));
    }
    let local = crate::config::load()?;
    let explicit = args.value("root").map(absolute).transpose()?;
    let root = crate::config::select_root(explicit.as_deref(), &local)?;
    store = Store::open(&root)?;
    store.lock_timeout_ms = args.number("lock-timeout-ms", 0, 30_000)? as u64;
    store.verification_max_age_days = local.verification_max_age_days.unwrap_or(30);
    let actor = args.value("actor").unwrap_or("local:unspecified");
    let store_id = Some(store.config.store_id.clone());
    let data = match command {
        "get" => {
            args.arity(2)?;
            args.allow(&["format"])?;
            let entry = store.get(&args.positional[1])?;
            match args.value("format") {
                Some("markdown") => {
                    if args.flag("json") {
                        return Err(invalid("markdown and JSON output are mutually exclusive"));
                    }
                    if std::io::stdout().is_terminal() {
                        return Err(ContextError::new(
                            "RAW_OUTPUT_REQUIRES_PIPE",
                            "raw Markdown requires a pipe or file",
                        ));
                    }
                    return Ok(Output::Raw(entry.raw_bytes));
                }
                None => {}
                _ => return Err(invalid("unsupported output format")),
            }
            let state =
                crate::format::verification_state(&entry.entry, store.verification_max_age_days);
            json!({"id":entry.entry.metadata.id,"revision":entry.revision,"raw_text":String::from_utf8(entry.raw_bytes).map_err(|_|invalid("entry is not UTF-8"))?,"metadata":entry.entry.metadata,"verification_state":state})
        }
        "put" => {
            args.arity(2)?;
            args.allow(&[
                "file",
                "expected-revision",
                "if-absent",
                "actor",
                "lock-timeout-ms",
            ])?;
            let result = crate::application::save_raw(
                &store,
                &args.positional[1],
                &candidate(args.required("file")?)?,
                condition(args)?,
                actor,
            )?;
            let state = store.get(&args.positional[1])?.verification_state;
            json!({"id":args.positional[1],"revision":result.revision,"persistence":result.persistence,"changed":result.changed,"verification_state":state})
        }
        "list" | "search" => {
            args.arity(if command == "list" { 1 } else { 2 })?;
            args.allow(&[
                "kind",
                "environment",
                "tag",
                "include-archived",
                "limit",
                "cursor",
            ])?;
            let filters = crate::query::ListQuery {
                kind: args.value("kind").map(str::to_owned),
                environment: args.value("environment").map(str::to_owned),
                tags: args.many("tag"),
                include_archived: args.flag("include-archived"),
                limit: args.number("limit", 20, 200)?,
                cursor: args.value("cursor").map(str::to_owned),
            };
            let result = if command == "list" {
                store.list(&filters)?
            } else {
                store.search(&crate::query::SearchQuery {
                    query: args.positional[1].clone(),
                    filters,
                })?
            };
            let warnings = serialize(&result.warnings)?
                .as_array()
                .cloned()
                .unwrap_or_default();
            return Ok(Output::Data {
                operation: command.into(),
                store_id,
                data: serialize(result)?,
                warnings,
            });
        }
        "archive" => {
            args.arity(2)?;
            args.allow(&["expected-revision", "actor", "lock-timeout-ms"])?;
            serialize(store.archive(
                &args.positional[1],
                args.required("expected-revision")?,
                actor,
            )?)?
        }
        "verify" => {
            args.arity(2)?;
            args.allow(&[
                "expected-revision",
                "actor",
                "lock-timeout-ms",
                "method",
                "observed-at",
                "evidence-ref",
            ])?;
            serialize(store.verify(
                &args.positional[1],
                args.required("expected-revision")?,
                args.required("method")?,
                args.required("evidence-ref")?,
                args.value("observed-at"),
                actor,
            )?)?
        }
        "edit" => {
            args.arity(2)?;
            args.allow(&["actor", "lock-timeout-ms"])?;
            serialize(crate::editor::edit(
                &store,
                &args.positional[1],
                actor,
                &local,
            )?)?
        }
        "doctor" => {
            args.arity(1)?;
            args.allow(&[])?;
            store.doctor()?
        }
        "status" => {
            args.arity(1)?;
            args.allow(&[])?;
            serialize(crate::git::status(&root).map_err(git_error)?)?
        }
        "history" => {
            args.arity(2)?;
            args.allow(&["limit"])?;
            serialize(
                crate::git::history(&root, &args.positional[1], args.number("limit", 20, 1000)?)
                    .map_err(git_error)?,
            )?
        }
        "restore" => {
            args.arity(2)?;
            args.allow(&[
                "from-commit",
                "expected-revision",
                "if-absent",
                "actor",
                "lock-timeout-ms",
            ])?;
            let (commit, saved) = crate::application::restore_entry(
                &store,
                &args.positional[1],
                args.required("from-commit")?,
                condition(args)?,
                actor,
            )?;
            json!({"id":args.positional[1],"from_commit":commit,"revision":saved.revision,"persistence":saved.persistence,"changed":saved.changed})
        }
        _ => return Err(invalid("unknown context command")),
    };
    Ok(output(command, store_id, data))
}

fn snapshot(args: &Arguments) -> Result<Output> {
    args.arity(2)?;
    let action = args.positional[1].as_str();
    let summary = match action {
        "export" => {
            args.allow(&["output", "id", "passphrase-stdin", "lock-timeout-ms"])?;
            let local = crate::config::load()?;
            let explicit = args.value("root").map(absolute).transpose()?;
            let root = crate::config::select_root(explicit.as_deref(), &local)?;
            let output = absolute(args.required("output")?)?;
            let pass = crate::passphrase::read(args.flag("passphrase-stdin"), true)?;
            crate::application::snapshot_export(
                &root,
                &output,
                &args.many("id"),
                &pass,
                args.number("lock-timeout-ms", 0, 30_000)? as u64,
            )?
        }
        "inspect" => {
            args.allow(&["input", "passphrase-stdin"])?;
            let input = absolute(args.required("input")?)?;
            let pass = crate::passphrase::read(args.flag("passphrase-stdin"), false)?;
            crate::application::snapshot_inspect(&input, &pass)?
        }
        "restore" => {
            args.allow(&[
                "input",
                "to",
                "expected-project",
                "expected-store-id",
                "passphrase-stdin",
            ])?;
            let input = absolute(args.required("input")?)?;
            let target = absolute(args.required("to")?)?;
            let pass = crate::passphrase::read(args.flag("passphrase-stdin"), false)?;
            crate::application::snapshot_restore(
                &input,
                &target,
                args.value("expected-project"),
                args.value("expected-store-id"),
                &pass,
            )?
        }
        _ => return Err(invalid("unknown snapshot command")),
    };
    let data = serialize(summary)?;
    Ok(output(
        &format!("snapshot {action}"),
        data.get("store_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        data,
    ))
}
