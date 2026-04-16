use super::*;

#[derive(Debug, serde::Deserialize)]
struct ChainStatusSyncSnapshot {
    consensus: ChainStatusConsensusSnapshot,
    execution_world_dir: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
struct ChainStatusConsensusSnapshot {
    committed_height: u64,
}

impl ViewerRuntimeLiveServer {
    pub(super) fn sync_chain_linked_runtime(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let Some(chain_status_bind) = self
            .config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(false);
        };

        let chain_status = fetch_chain_status_snapshot(chain_status_bind)?;
        if chain_status.consensus.committed_height <= self.last_chain_committed_height {
            return Ok(false);
        }

        self.last_chain_committed_height = chain_status.consensus.committed_height;
        let baseline_logical_time = self.world.state().time;
        let baseline_event_seq = latest_runtime_event_seq(&self.world);
        self.world = load_chain_execution_world(chain_status.execution_world_dir.as_path())?;

        let delta_logical_time = self
            .world
            .state()
            .time
            .saturating_sub(baseline_logical_time);
        let delta_event_seq =
            latest_runtime_event_seq(&self.world).saturating_sub(baseline_event_seq);
        if delta_logical_time == 0 && delta_event_seq == 0 {
            return Ok(false);
        }
        self.confirm_player_gameplay_progress();

        let mapped_events: Vec<_> = self
            .world
            .journal()
            .events
            .iter()
            .filter(|event| event.id > baseline_event_seq)
            .map(|runtime_event| map_runtime_event(runtime_event, &self.snapshot_config))
            .collect();
        let pending_batch = self.register_authoritative_batch(mapped_events.as_slice())?;
        let batch_finality_updates =
            self.advance_authoritative_batch_finality(self.world.state().time)?;

        if session.subscribed.contains(&ViewerStream::Events) {
            for event in &mapped_events {
                if session.event_allowed(event) {
                    send_response(
                        writer,
                        &ViewerResponse::Event {
                            event: event.clone(),
                        },
                    )?;
                }
            }
            send_response(
                writer,
                &ViewerResponse::AuthoritativeBatch {
                    batch: pending_batch,
                },
            )?;
            for batch in batch_finality_updates {
                send_response(writer, &ViewerResponse::AuthoritativeBatch { batch })?;
            }
        }

        if session.subscribed.contains(&ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot();
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }

        session.metrics = runtime_metrics(&self.world);
        if session.subscribed.contains(&ViewerStream::Metrics) {
            send_response(
                writer,
                &ViewerResponse::Metrics {
                    time: Some(self.world.state().time),
                    metrics: session.metrics.clone(),
                },
            )?;
        }

        Ok(true)
    }
}

fn fetch_chain_status_snapshot(
    chain_status_bind: &str,
) -> Result<ChainStatusSyncSnapshot, ViewerRuntimeLiveServerError> {
    let mut stream = TcpStream::connect(chain_status_bind)?;
    stream.set_read_timeout(Some(Duration::from_millis(300)))?;
    stream.set_write_timeout(Some(Duration::from_millis(300)))?;
    let request = format!(
        "GET /v1/chain/status HTTP/1.1\r\nHost: {chain_status_bind}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let Some(body_start) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err(ViewerRuntimeLiveServerError::Serde(
            "chain status response missing HTTP body".to_string(),
        ));
    };
    let header = String::from_utf8_lossy(&response[..body_start]);
    if !header.starts_with("HTTP/1.1 200") && !header.starts_with("HTTP/1.0 200") {
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "chain status request returned non-200 response: {}",
            header.lines().next().unwrap_or("unknown_status")
        )));
    }
    serde_json::from_slice(&response[body_start + 4..])
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))
}

fn load_chain_execution_world(
    execution_world_dir: &Path,
) -> Result<RuntimeWorld, ViewerRuntimeLiveServerError> {
    let snapshot_path = execution_world_dir.join("snapshot.json");
    let journal_path = execution_world_dir.join("journal.json");
    if !snapshot_path.exists() || !journal_path.exists() {
        return Ok(RuntimeWorld::new_production_hardened());
    }

    RuntimeWorld::load_from_dir(execution_world_dir)
        .map(|world| {
            world.with_release_security_policy(ReleaseSecurityPolicy::production_hardened())
        })
        .map_err(ViewerRuntimeLiveServerError::Runtime)
}
