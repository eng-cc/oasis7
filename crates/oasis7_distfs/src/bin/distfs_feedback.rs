use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oasis7_distfs::{
    FeedbackAppendRequest, FeedbackAttachment, FeedbackCreateRequest, FeedbackStore,
    FeedbackStoreConfig, FeedbackTombstoneRequest, LocalCasStore,
    public_key_hex_from_signing_key_hex, sign_feedback_append_request,
    sign_feedback_create_request, sign_feedback_tombstone_request,
};

#[derive(Debug)]
enum Command {
    Submit(SubmitOptions),
    Append(AppendOptions),
    Tombstone(TombstoneOptions),
    List(ListOptions),
    Read(ReadOptions),
}

#[derive(Debug)]
struct SubmitOptions {
    root: PathBuf,
    feedback_id: String,
    signing_key_hex: String,
    submit_ip: String,
    category: String,
    platform: String,
    game_version: String,
    content: String,
    attachments: Vec<FeedbackAttachment>,
    nonce: Option<String>,
    timestamp_ms: Option<i64>,
    ttl_ms: i64,
}

#[derive(Debug)]
struct AppendOptions {
    root: PathBuf,
    feedback_id: String,
    signing_key_hex: String,
    submit_ip: String,
    content: String,
    nonce: Option<String>,
    timestamp_ms: Option<i64>,
    ttl_ms: i64,
}

#[derive(Debug)]
struct TombstoneOptions {
    root: PathBuf,
    feedback_id: String,
    signing_key_hex: String,
    submit_ip: String,
    reason: String,
    nonce: Option<String>,
    timestamp_ms: Option<i64>,
    ttl_ms: i64,
}

#[derive(Debug)]
struct ListOptions {
    root: PathBuf,
}

#[derive(Debug)]
struct ReadOptions {
    root: PathBuf,
    feedback_id: String,
}

fn usage() -> &'static str {
    "Usage:
  distfs_feedback <command> [options]

Commands:
  submit      create feedback root record
  append      append content event to existing feedback
  tombstone   logical delete by tombstone event
  list        list all feedback public views
  read        read one feedback public view

Shared option:
  --root <path>      DistFS root directory

submit options:
  --feedback-id <id>
  --signing-key-hex <32-byte-hex>
  --ip <ipv4|ipv6>
  --category <text>
  --platform <text>
  --game-version <text>
  --content <text>
  --attachment <content_hash:size_bytes:mime_type>   (repeatable)
  --nonce <text>                                     (optional)
  --timestamp-ms <int>                               (optional)
  --ttl-ms <int>                                     (optional, default 60000)

append options:
  --feedback-id <id>
  --signing-key-hex <32-byte-hex>
  --ip <ipv4|ipv6>
  --content <text>
  --nonce <text>                                     (optional)
  --timestamp-ms <int>                               (optional)
  --ttl-ms <int>                                     (optional, default 60000)

tombstone options:
  --feedback-id <id>
  --signing-key-hex <32-byte-hex>
  --ip <ipv4|ipv6>
  --reason <text>
  --nonce <text>                                     (optional)
  --timestamp-ms <int>                               (optional)
  --ttl-ms <int>                                     (optional, default 60000)

list options:
  --root <path>

read options:
  --root <path>
  --feedback-id <id>"
}

fn parse_args() -> Result<Command, String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("missing command".to_string());
    };
    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "submit" => Ok(Command::Submit(parse_submit_options(rest)?)),
        "append" => Ok(Command::Append(parse_append_options(rest)?)),
        "tombstone" => Ok(Command::Tombstone(parse_tombstone_options(rest)?)),
        "list" => Ok(Command::List(parse_list_options(rest)?)),
        "read" => Ok(Command::Read(parse_read_options(rest)?)),
        "-h" | "--help" => Err(usage().to_string()),
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_submit_options(args: Vec<String>) -> Result<SubmitOptions, String> {
    let mut root: Option<PathBuf> = None;
    let mut feedback_id: Option<String> = None;
    let mut signing_key_hex: Option<String> = None;
    let mut submit_ip: Option<String> = None;
    let mut category: Option<String> = None;
    let mut platform: Option<String> = None;
    let mut game_version: Option<String> = None;
    let mut content: Option<String> = None;
    let mut attachments = Vec::new();
    let mut nonce: Option<String> = None;
    let mut timestamp_ms: Option<i64> = None;
    let mut ttl_ms: i64 = 60_000;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(next_arg_value(&mut iter, "--root")?)),
            "--feedback-id" => feedback_id = Some(next_arg_value(&mut iter, "--feedback-id")?),
            "--signing-key-hex" => {
                signing_key_hex = Some(next_arg_value(&mut iter, "--signing-key-hex")?)
            }
            "--ip" => submit_ip = Some(next_arg_value(&mut iter, "--ip")?),
            "--category" => category = Some(next_arg_value(&mut iter, "--category")?),
            "--platform" => platform = Some(next_arg_value(&mut iter, "--platform")?),
            "--game-version" => game_version = Some(next_arg_value(&mut iter, "--game-version")?),
            "--content" => content = Some(next_arg_value(&mut iter, "--content")?),
            "--attachment" => attachments.push(parse_attachment(&next_arg_value(
                &mut iter,
                "--attachment",
            )?)?),
            "--nonce" => nonce = Some(next_arg_value(&mut iter, "--nonce")?),
            "--timestamp-ms" => {
                timestamp_ms = Some(parse_i64_arg(&next_arg_value(
                    &mut iter,
                    "--timestamp-ms",
                )?)?)
            }
            "--ttl-ms" => ttl_ms = parse_i64_arg(&next_arg_value(&mut iter, "--ttl-ms")?)?,
            other => return Err(format!("unknown submit option: {other}")),
        }
    }
    Ok(SubmitOptions {
        root: required_opt(root, "--root")?,
        feedback_id: required_opt(feedback_id, "--feedback-id")?,
        signing_key_hex: required_opt(signing_key_hex, "--signing-key-hex")?,
        submit_ip: required_opt(submit_ip, "--ip")?,
        category: required_opt(category, "--category")?,
        platform: required_opt(platform, "--platform")?,
        game_version: required_opt(game_version, "--game-version")?,
        content: required_opt(content, "--content")?,
        attachments,
        nonce,
        timestamp_ms,
        ttl_ms,
    })
}

fn parse_append_options(args: Vec<String>) -> Result<AppendOptions, String> {
    let mut root: Option<PathBuf> = None;
    let mut feedback_id: Option<String> = None;
    let mut signing_key_hex: Option<String> = None;
    let mut submit_ip: Option<String> = None;
    let mut content: Option<String> = None;
    let mut nonce: Option<String> = None;
    let mut timestamp_ms: Option<i64> = None;
    let mut ttl_ms: i64 = 60_000;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(next_arg_value(&mut iter, "--root")?)),
            "--feedback-id" => feedback_id = Some(next_arg_value(&mut iter, "--feedback-id")?),
            "--signing-key-hex" => {
                signing_key_hex = Some(next_arg_value(&mut iter, "--signing-key-hex")?)
            }
            "--ip" => submit_ip = Some(next_arg_value(&mut iter, "--ip")?),
            "--content" => content = Some(next_arg_value(&mut iter, "--content")?),
            "--nonce" => nonce = Some(next_arg_value(&mut iter, "--nonce")?),
            "--timestamp-ms" => {
                timestamp_ms = Some(parse_i64_arg(&next_arg_value(
                    &mut iter,
                    "--timestamp-ms",
                )?)?)
            }
            "--ttl-ms" => ttl_ms = parse_i64_arg(&next_arg_value(&mut iter, "--ttl-ms")?)?,
            other => return Err(format!("unknown append option: {other}")),
        }
    }
    Ok(AppendOptions {
        root: required_opt(root, "--root")?,
        feedback_id: required_opt(feedback_id, "--feedback-id")?,
        signing_key_hex: required_opt(signing_key_hex, "--signing-key-hex")?,
        submit_ip: required_opt(submit_ip, "--ip")?,
        content: required_opt(content, "--content")?,
        nonce,
        timestamp_ms,
        ttl_ms,
    })
}

fn parse_tombstone_options(args: Vec<String>) -> Result<TombstoneOptions, String> {
    let mut root: Option<PathBuf> = None;
    let mut feedback_id: Option<String> = None;
    let mut signing_key_hex: Option<String> = None;
    let mut submit_ip: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut nonce: Option<String> = None;
    let mut timestamp_ms: Option<i64> = None;
    let mut ttl_ms: i64 = 60_000;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(next_arg_value(&mut iter, "--root")?)),
            "--feedback-id" => feedback_id = Some(next_arg_value(&mut iter, "--feedback-id")?),
            "--signing-key-hex" => {
                signing_key_hex = Some(next_arg_value(&mut iter, "--signing-key-hex")?)
            }
            "--ip" => submit_ip = Some(next_arg_value(&mut iter, "--ip")?),
            "--reason" => reason = Some(next_arg_value(&mut iter, "--reason")?),
            "--nonce" => nonce = Some(next_arg_value(&mut iter, "--nonce")?),
            "--timestamp-ms" => {
                timestamp_ms = Some(parse_i64_arg(&next_arg_value(
                    &mut iter,
                    "--timestamp-ms",
                )?)?)
            }
            "--ttl-ms" => ttl_ms = parse_i64_arg(&next_arg_value(&mut iter, "--ttl-ms")?)?,
            other => return Err(format!("unknown tombstone option: {other}")),
        }
    }
    Ok(TombstoneOptions {
        root: required_opt(root, "--root")?,
        feedback_id: required_opt(feedback_id, "--feedback-id")?,
        signing_key_hex: required_opt(signing_key_hex, "--signing-key-hex")?,
        submit_ip: required_opt(submit_ip, "--ip")?,
        reason: required_opt(reason, "--reason")?,
        nonce,
        timestamp_ms,
        ttl_ms,
    })
}

fn parse_list_options(args: Vec<String>) -> Result<ListOptions, String> {
    let mut root: Option<PathBuf> = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(next_arg_value(&mut iter, "--root")?)),
            other => return Err(format!("unknown list option: {other}")),
        }
    }
    Ok(ListOptions {
        root: required_opt(root, "--root")?,
    })
}

fn parse_read_options(args: Vec<String>) -> Result<ReadOptions, String> {
    let mut root: Option<PathBuf> = None;
    let mut feedback_id: Option<String> = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(next_arg_value(&mut iter, "--root")?)),
            "--feedback-id" => feedback_id = Some(next_arg_value(&mut iter, "--feedback-id")?),
            other => return Err(format!("unknown read option: {other}")),
        }
    }
    Ok(ReadOptions {
        root: required_opt(root, "--root")?,
        feedback_id: required_opt(feedback_id, "--feedback-id")?,
    })
}

fn parse_attachment(value: &str) -> Result<FeedbackAttachment, String> {
    let mut parts = value.splitn(3, ':');
    let content_hash = parts
        .next()
        .ok_or_else(|| "attachment content_hash missing".to_string())?;
    let size_bytes = parts
        .next()
        .ok_or_else(|| "attachment size_bytes missing".to_string())?;
    let mime_type = parts
        .next()
        .ok_or_else(|| "attachment mime_type missing".to_string())?;
    let size_bytes = size_bytes.parse::<u64>().map_err(|error| {
        format!("invalid attachment size_bytes value={size_bytes} error={error}")
    })?;
    Ok(FeedbackAttachment {
        content_hash: content_hash.to_string(),
        size_bytes,
        mime_type: mime_type.to_string(),
    })
}

fn next_arg_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_i64_arg(value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid int value={value}: {error}"))
}

fn required_opt<T>(value: Option<T>, label: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("{label} is required"))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn build_nonce(prefix: &str, feedback_id: &str, timestamp_ms: i64) -> String {
    format!("{prefix}-{feedback_id}-{timestamp_ms}")
}

fn run() -> Result<(), String> {
    let command = parse_args()?;
    match command {
        Command::Submit(options) => {
            let store = FeedbackStore::new(
                LocalCasStore::new(options.root.as_path()),
                FeedbackStoreConfig::default(),
            );
            let timestamp_ms = options.timestamp_ms.unwrap_or_else(now_unix_ms);
            let expires_at_ms = timestamp_ms.saturating_add(options.ttl_ms);
            let feedback_id_for_nonce = options.feedback_id.clone();
            let author_public_key_hex =
                public_key_hex_from_signing_key_hex(options.signing_key_hex.as_str())
                    .map_err(|error| format!("derive pubkey failed: {error:?}"))?;
            let mut request = FeedbackCreateRequest {
                feedback_id: options.feedback_id,
                author_public_key_hex,
                submit_ip: options.submit_ip,
                category: options.category,
                platform: options.platform,
                game_version: options.game_version,
                content: options.content,
                attachments: options.attachments,
                nonce: options.nonce.unwrap_or_else(|| {
                    build_nonce("create", feedback_id_for_nonce.as_str(), timestamp_ms)
                }),
                timestamp_ms,
                expires_at_ms,
                signature_hex: String::new(),
            };
            request.signature_hex =
                sign_feedback_create_request(&request, options.signing_key_hex.as_str())
                    .map_err(|error| format!("sign create request failed: {error:?}"))?;
            let receipt = store
                .submit_feedback(request)
                .map_err(|error| format!("submit failed: {error:?}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
            );
        }
        Command::Append(options) => {
            let store = FeedbackStore::new(
                LocalCasStore::new(options.root.as_path()),
                FeedbackStoreConfig::default(),
            );
            let timestamp_ms = options.timestamp_ms.unwrap_or_else(now_unix_ms);
            let expires_at_ms = timestamp_ms.saturating_add(options.ttl_ms);
            let feedback_id_for_nonce = options.feedback_id.clone();
            let actor_public_key_hex =
                public_key_hex_from_signing_key_hex(options.signing_key_hex.as_str())
                    .map_err(|error| format!("derive pubkey failed: {error:?}"))?;
            let mut request = FeedbackAppendRequest {
                feedback_id: options.feedback_id,
                actor_public_key_hex,
                submit_ip: options.submit_ip,
                content: options.content,
                nonce: options.nonce.unwrap_or_else(|| {
                    build_nonce("append", feedback_id_for_nonce.as_str(), timestamp_ms)
                }),
                timestamp_ms,
                expires_at_ms,
                signature_hex: String::new(),
            };
            request.signature_hex =
                sign_feedback_append_request(&request, options.signing_key_hex.as_str())
                    .map_err(|error| format!("sign append request failed: {error:?}"))?;
            let receipt = store
                .append_feedback(request)
                .map_err(|error| format!("append failed: {error:?}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
            );
        }
        Command::Tombstone(options) => {
            let store = FeedbackStore::new(
                LocalCasStore::new(options.root.as_path()),
                FeedbackStoreConfig::default(),
            );
            let timestamp_ms = options.timestamp_ms.unwrap_or_else(now_unix_ms);
            let expires_at_ms = timestamp_ms.saturating_add(options.ttl_ms);
            let feedback_id_for_nonce = options.feedback_id.clone();
            let actor_public_key_hex =
                public_key_hex_from_signing_key_hex(options.signing_key_hex.as_str())
                    .map_err(|error| format!("derive pubkey failed: {error:?}"))?;
            let mut request = FeedbackTombstoneRequest {
                feedback_id: options.feedback_id,
                actor_public_key_hex,
                submit_ip: options.submit_ip,
                reason: options.reason,
                nonce: options.nonce.unwrap_or_else(|| {
                    build_nonce("tombstone", feedback_id_for_nonce.as_str(), timestamp_ms)
                }),
                timestamp_ms,
                expires_at_ms,
                signature_hex: String::new(),
            };
            request.signature_hex =
                sign_feedback_tombstone_request(&request, options.signing_key_hex.as_str())
                    .map_err(|error| format!("sign tombstone request failed: {error:?}"))?;
            let receipt = store
                .tombstone_feedback(request)
                .map_err(|error| format!("tombstone failed: {error:?}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
            );
        }
        Command::List(options) => {
            let store = FeedbackStore::new(
                LocalCasStore::new(options.root.as_path()),
                FeedbackStoreConfig::default(),
            );
            let list = store
                .list_feedback_public()
                .map_err(|error| format!("list failed: {error:?}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&list).map_err(|error| error.to_string())?
            );
        }
        Command::Read(options) => {
            let store = FeedbackStore::new(
                LocalCasStore::new(options.root.as_path()),
                FeedbackStoreConfig::default(),
            );
            let view = store
                .read_feedback_public(options.feedback_id.as_str())
                .map_err(|error| format!("read failed: {error:?}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&view).map_err(|error| error.to_string())?
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        eprintln!("{}", usage());
        std::process::exit(2);
    }
}
