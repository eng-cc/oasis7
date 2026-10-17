use super::*;

pub(super) fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn write_jsonl(path: &Path, records: &[StepTraceRecord]) -> Result<(), String> {
    let mut file =
        File::create(path).map_err(|err| format!("create {} failed: {err}", path.display()))?;
    for record in records {
        let line = serde_json::to_string(record)
            .map_err(|err| format!("serialize record failed: {err}"))?;
        writeln!(file, "{line}")
            .map_err(|err| format!("write {} failed: {err}", path.display()))?;
    }
    Ok(())
}

pub(super) fn write_json(path: &Path, summary: &SampleSummary) -> Result<(), String> {
    let content = serde_json::to_string_pretty(summary)
        .map_err(|err| format!("serialize summary failed: {err}"))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|err| format!("write {} failed: {err}", path.display()))
}

pub(super) fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        match arg {
            "--provider" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--provider requires a value".to_string())?;
                options.provider = BenchProviderKind::parse(raw)
                    .ok_or_else(|| format!("invalid --provider: {raw}"))?;
            }
            "--scenario" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--scenario requires a value".to_string())?;
                options.scenario = WorldScenario::parse(raw)
                    .ok_or_else(|| format!("invalid --scenario: {raw}"))?;
            }
            "--scenario-id" => {
                options.scenario_id = iter
                    .next()
                    .ok_or_else(|| "--scenario-id requires a value".to_string())?
                    .to_string();
            }
            "--parity-tier" => {
                options.parity_tier = iter
                    .next()
                    .ok_or_else(|| "--parity-tier requires a value".to_string())?
                    .to_string();
            }
            "--benchmark-run-id" => {
                options.benchmark_run_id = iter
                    .next()
                    .ok_or_else(|| "--benchmark-run-id requires a value".to_string())?
                    .to_string();
            }
            "--fixture-id" => {
                options.fixture_id = Some(
                    iter.next()
                        .ok_or_else(|| "--fixture-id requires a value".to_string())?
                        .to_string(),
                );
            }
            "--protocol-version" => {
                options.protocol_version = iter
                    .next()
                    .ok_or_else(|| "--protocol-version requires a value".to_string())?
                    .to_string();
            }
            "--adapter-version" => {
                options.adapter_version = iter
                    .next()
                    .ok_or_else(|| "--adapter-version requires a value".to_string())?
                    .to_string();
            }
            "--ticks" => {
                options.ticks = parse_u64(
                    iter.next()
                        .ok_or_else(|| "--ticks requires a value".to_string())?,
                    "--ticks",
                )?;
            }
            "--timeout-ms" => {
                options.timeout_ms = parse_u64(
                    iter.next()
                        .ok_or_else(|| "--timeout-ms requires a value".to_string())?,
                    "--timeout-ms",
                )?;
            }
            "--out-dir" => {
                options.out_dir = PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "--out-dir requires a value".to_string())?,
                );
            }
            "--agent-provider-url" => {
                options.provider_base_url = Some(
                    iter.next()
                        .ok_or_else(|| "--agent-provider-url requires a value".to_string())?
                        .to_string(),
                );
            }
            "--agent-provider-auth-token" => {
                options.provider_auth_token = Some(
                    iter.next()
                        .ok_or_else(|| "--agent-provider-auth-token requires a value".to_string())?
                        .to_string(),
                );
            }
            "--agent-provider-connect-timeout-ms" => {
                options.agent_provider_connect_timeout_ms = parse_u64(
                    iter.next().ok_or_else(|| {
                        "--agent-provider-connect-timeout-ms requires a value".to_string()
                    })?,
                    "--agent-provider-connect-timeout-ms",
                )?;
            }
            "--agent-provider-profile" => {
                options.agent_provider_profile = iter
                    .next()
                    .ok_or_else(|| "--agent-provider-profile requires a value".to_string())?
                    .to_string();
            }
            "--execution-mode" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--execution-mode requires a value".to_string())?;
                options.execution_mode = ProviderExecutionMode::parse(raw).ok_or_else(|| {
                    format!(
                        "invalid --execution-mode `{raw}`: expected player_parity or headless_agent"
                    )
                })?;
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }

    if options.scenario_id.trim().is_empty() {
        return Err("--scenario-id cannot be empty".to_string());
    }
    if options.parity_tier.trim().is_empty() {
        return Err("--parity-tier cannot be empty".to_string());
    }
    if options.benchmark_run_id.trim().is_empty() {
        return Err("--benchmark-run-id cannot be empty".to_string());
    }
    if options.out_dir.as_os_str().is_empty() {
        return Err("--out-dir cannot be empty".to_string());
    }
    if options.provider == BenchProviderKind::ProviderLoopbackHttp
        && options
            .provider_base_url
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err("--agent-provider-url is required for provider_loopback_http".to_string());
    }
    if options.provider == BenchProviderKind::ProviderLoopbackHttp
        && options.agent_provider_profile.trim().is_empty()
    {
        return Err("--agent-provider-profile cannot be empty".to_string());
    }
    if options.provider == BenchProviderKind::Builtin
        && options.execution_mode != ProviderExecutionMode::HeadlessAgent
    {
        return Err(
            "--execution-mode=player_parity is only supported with --provider provider_loopback_http"
                .to_string(),
        );
    }
    Ok(options)
}

fn parse_u64(raw: &str, flag: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .map_err(|err| format!("invalid {flag}: {err}"))
}

pub(super) fn print_help() {
    println!(
        "Usage: oasis7_provider_parity_bench [options]\n\n\
Run one parity benchmark sample for builtin or the loopback provider and emit\n\
raw jsonl + single-sample summary json following the parity benchmark contract.\n\n\
Options:\n\
  --provider <builtin|provider_loopback_http|provider_local_bridge>\n\
                               provider_local_bridge is accepted as an alias of provider_loopback_http\n\
  --scenario <name>\n\
  --scenario-id <id>\n\
  --parity-tier <P0|P1|P2>\n\
  --benchmark-run-id <id>\n\
  --fixture-id <id>\n\
  --ticks <n>\n\
  --timeout-ms <n>\n\
  --out-dir <path>\n\
  --agent-provider-url <url>\n\
  --agent-provider-auth-token <token>\n\
  --agent-provider-connect-timeout-ms <n>\n\
  --agent-provider-profile <profile>\n\
  --execution-mode <player_parity|headless_agent>\n\
  -h, --help\n"
    );
}
