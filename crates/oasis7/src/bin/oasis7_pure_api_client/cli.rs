use super::*;

pub(super) fn parse_cli(args: &mut ArgCursor) -> Result<CliConfig, String> {
    let mut addr = DEFAULT_ADDR.to_string();
    let mut client = DEFAULT_CLIENT.to_string();
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    loop {
        match args.peek() {
            Some("--addr") => {
                args.next();
                addr = args.value("--addr")?;
            }
            Some("--client") => {
                args.next();
                client = args.value("--client")?;
            }
            Some("--timeout-ms") => {
                args.next();
                timeout_ms = parse_u64_flag(args.value("--timeout-ms")?, "--timeout-ms")?;
            }
            Some("-h") | Some("--help") => return Err(usage()),
            _ => break,
        }
    }
    let subcommand = args.next().ok_or_else(usage)?.to_ascii_lowercase();
    let command = match subcommand.as_str() {
        "keygen" => Command::Keygen,
        "agency-control" => {
            let mut request_json = None;
            while let Some(flag) = args.peek() {
                match flag {
                    "--request-json" => {
                        args.next();
                        request_json = Some(args.value("--request-json")?);
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown agency-control flag `{flag}`")),
                }
            }
            Command::AgencyControl {
                request_json: request_json
                    .ok_or_else(|| "agency-control requires --request-json".to_string())?,
            }
        }
        "snapshot" => {
            let mut player_gameplay_only = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-gameplay-only" => {
                        args.next();
                        player_gameplay_only = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown snapshot flag `{flag}`")),
                }
            }
            Command::Snapshot {
                player_gameplay_only,
            }
        }
        "step" => {
            let mut count = 1usize;
            let mut request_id = None;
            let mut include_events = false;
            let mut include_metrics = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--count" => {
                        args.next();
                        count = parse_usize_flag(args.value("--count")?, "--count")?;
                    }
                    "--request-id" => {
                        args.next();
                        request_id =
                            Some(parse_u64_flag(args.value("--request-id")?, "--request-id")?);
                    }
                    "--events" => {
                        args.next();
                        include_events = true;
                    }
                    "--metrics" => {
                        args.next();
                        include_metrics = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown step flag `{flag}`")),
                }
            }
            Command::Step {
                count,
                request_id,
                include_events,
                include_metrics,
            }
        }
        "play" => Command::Play {
            include_events: parse_bool_flag(args, "--events")?,
            include_metrics: parse_bool_flag(args, "--metrics")?,
        },
        "pause" => Command::Pause {
            include_events: parse_bool_flag(args, "--events")?,
            include_metrics: parse_bool_flag(args, "--metrics")?,
        },
        "chat" => {
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut message = None;
            let mut intent_tick = None;
            let mut intent_seq = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--message" => {
                        args.next();
                        message = Some(args.value("--message")?);
                    }
                    "--intent-tick" => {
                        args.next();
                        intent_tick = Some(parse_u64_flag(
                            args.value("--intent-tick")?,
                            "--intent-tick",
                        )?);
                    }
                    "--intent-seq" => {
                        args.next();
                        intent_seq =
                            Some(parse_u64_flag(args.value("--intent-seq")?, "--intent-seq")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown chat flag `{flag}`")),
                }
            }
            Command::Chat {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                message: required_flag(message, "--message")?,
                intent_tick,
                intent_seq,
                with_snapshot,
            }
        }
        "gameplay-action" => {
            let mut action_id = None;
            let mut target_agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--action-id" => {
                        args.next();
                        action_id = Some(args.value("--action-id")?);
                    }
                    "--target-agent-id" => {
                        args.next();
                        target_agent_id = Some(args.value("--target-agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown gameplay-action flag `{flag}`")),
                }
            }
            Command::GameplayAction {
                action_id: required_flag(action_id, "--action-id")?,
                target_agent_id: required_flag(target_agent_id, "--target-agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                with_snapshot,
            }
        }
        "prompt-apply" | "prompt-preview" => {
            let preview = subcommand == "prompt-preview";
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut expected_version = None;
            let mut updated_by = None;
            let mut system_prompt_override = None;
            let mut short_term_goal_override = None;
            let mut long_term_goal_override = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--expected-version" => {
                        args.next();
                        expected_version = Some(parse_u64_flag(
                            args.value("--expected-version")?,
                            "--expected-version",
                        )?);
                    }
                    "--updated-by" => {
                        args.next();
                        updated_by = Some(args.value("--updated-by")?);
                    }
                    "--system-prompt" => {
                        args.next();
                        system_prompt_override = Some(Some(args.value("--system-prompt")?));
                    }
                    "--clear-system-prompt" => {
                        args.next();
                        system_prompt_override = Some(None);
                    }
                    "--short-term-goal" => {
                        args.next();
                        short_term_goal_override = Some(Some(args.value("--short-term-goal")?));
                    }
                    "--clear-short-term-goal" => {
                        args.next();
                        short_term_goal_override = Some(None);
                    }
                    "--long-term-goal" => {
                        args.next();
                        long_term_goal_override = Some(Some(args.value("--long-term-goal")?));
                    }
                    "--clear-long-term-goal" => {
                        args.next();
                        long_term_goal_override = Some(None);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown prompt flag `{flag}`")),
                }
            }
            Command::PromptApply {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                expected_version,
                updated_by,
                system_prompt_override,
                short_term_goal_override,
                long_term_goal_override,
                preview,
                with_snapshot,
            }
        }
        "prompt-rollback" => {
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut to_version = None;
            let mut expected_version = None;
            let mut updated_by = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--to-version" => {
                        args.next();
                        to_version =
                            Some(parse_u64_flag(args.value("--to-version")?, "--to-version")?);
                    }
                    "--expected-version" => {
                        args.next();
                        expected_version = Some(parse_u64_flag(
                            args.value("--expected-version")?,
                            "--expected-version",
                        )?);
                    }
                    "--updated-by" => {
                        args.next();
                        updated_by = Some(args.value("--updated-by")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown prompt-rollback flag `{flag}`")),
                }
            }
            Command::PromptRollback {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                to_version: required_flag(to_version, "--to-version")?,
                expected_version,
                updated_by,
                with_snapshot,
            }
        }
        "register-session" => {
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut requested_agent_id = None;
            let mut registration_grant_file = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--requested-agent-id" => {
                        args.next();
                        requested_agent_id = Some(args.value("--requested-agent-id")?);
                    }
                    "--registration-grant-file" => {
                        args.next();
                        registration_grant_file = Some(args.value("--registration-grant-file")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown register-session flag `{flag}`")),
                }
            }
            Command::RegisterSession {
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                requested_agent_id,
                registration_grant_file,
                with_snapshot,
            }
        }
        "reconnect-sync" => {
            let mut player_id = None;
            let mut session_pubkey = None;
            let mut last_known_log_cursor = None;
            let mut expected_reorg_epoch = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--session-pubkey" => {
                        args.next();
                        session_pubkey = Some(args.value("--session-pubkey")?);
                    }
                    "--last-known-log-cursor" => {
                        args.next();
                        last_known_log_cursor = Some(parse_u64_flag(
                            args.value("--last-known-log-cursor")?,
                            "--last-known-log-cursor",
                        )?);
                    }
                    "--expected-reorg-epoch" => {
                        args.next();
                        expected_reorg_epoch = Some(parse_u64_flag(
                            args.value("--expected-reorg-epoch")?,
                            "--expected-reorg-epoch",
                        )?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown reconnect-sync flag `{flag}`")),
                }
            }
            Command::ReconnectSync {
                player_id: required_flag(player_id, "--player-id")?,
                session_pubkey,
                last_known_log_cursor,
                expected_reorg_epoch,
                with_snapshot,
            }
        }
        "rotate-session" => {
            let mut player_id = None;
            let mut old_session_pubkey = None;
            let mut new_session_pubkey = None;
            let mut rotate_reason = None;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--old-session-pubkey" => {
                        args.next();
                        old_session_pubkey = Some(args.value("--old-session-pubkey")?);
                    }
                    "--new-session-pubkey" => {
                        args.next();
                        new_session_pubkey = Some(args.value("--new-session-pubkey")?);
                    }
                    "--rotate-reason" => {
                        args.next();
                        rotate_reason = Some(args.value("--rotate-reason")?);
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown rotate-session flag `{flag}`")),
                }
            }
            Command::RotateSession {
                player_id: required_flag(player_id, "--player-id")?,
                old_session_pubkey: required_flag(old_session_pubkey, "--old-session-pubkey")?,
                new_session_pubkey: required_flag(new_session_pubkey, "--new-session-pubkey")?,
                rotate_reason: required_flag(rotate_reason, "--rotate-reason")?,
            }
        }
        "revoke-session" => {
            let mut player_id = None;
            let mut session_pubkey = None;
            let mut revoke_reason = None;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--session-pubkey" => {
                        args.next();
                        session_pubkey = Some(args.value("--session-pubkey")?);
                    }
                    "--revoke-reason" => {
                        args.next();
                        revoke_reason = Some(args.value("--revoke-reason")?);
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown revoke-session flag `{flag}`")),
                }
            }
            Command::RevokeSession {
                player_id: required_flag(player_id, "--player-id")?,
                session_pubkey,
                revoke_reason: required_flag(revoke_reason, "--revoke-reason")?,
            }
        }
        _ => return Err(format!("unknown subcommand `{subcommand}`\n\n{}", usage())),
    };

    Ok(CliConfig {
        addr,
        client,
        timeout_ms,
        command,
    })
}
