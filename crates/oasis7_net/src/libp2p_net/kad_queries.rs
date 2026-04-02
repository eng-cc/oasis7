use std::collections::HashSet;

use futures::channel::oneshot;
use libp2p::kad;
use libp2p::PeerId;

use crate::error::WorldError;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};

use super::peer_record::decode_peer_record;

pub(super) enum PendingDhtQuery {
    PublishProvider {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetProviders {
        response: Option<oneshot::Sender<Result<Vec<ProviderRecord>, WorldError>>>,
        providers: HashSet<PeerId>,
        error: Option<WorldError>,
    },
    PutWorldHead {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetWorldHead {
        response: Option<oneshot::Sender<Result<Option<WorldHeadAnnounce>, WorldError>>>,
        head: Option<WorldHeadAnnounce>,
        error: Option<WorldError>,
    },
    PutMembershipDirectory {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetMembershipDirectory {
        response: Option<oneshot::Sender<Result<Option<MembershipDirectorySnapshot>, WorldError>>>,
        snapshot: Option<MembershipDirectorySnapshot>,
        error: Option<WorldError>,
    },
    PutPeerRecord {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetPeerRecord {
        response: Option<oneshot::Sender<Result<Option<SignedPeerRecord>, WorldError>>>,
        record: Option<SignedPeerRecord>,
        error: Option<WorldError>,
    },
    DiscoverPeers {
        peers: HashSet<PeerId>,
        error: Option<WorldError>,
    },
    DiscoverPeerRecord {
        peer_id: PeerId,
        record: Option<SignedPeerRecord>,
        error: Option<WorldError>,
    },
}

pub(super) enum DhtProgressAction {
    None,
    DiscoverPeers(Vec<PeerId>),
    DiscoveryError(WorldError),
    DiscoverPeerRecord {
        peer_id: PeerId,
        result: Result<Option<SignedPeerRecord>, WorldError>,
    },
}

pub(super) fn handle_dht_progress(
    pending: &mut PendingDhtQuery,
    result: kad::QueryResult,
    is_last: bool,
) -> DhtProgressAction {
    match pending {
        PendingDhtQuery::PublishProvider { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::StartProviding(Ok(_))
                    | kad::QueryResult::RepublishProvider(Ok(_)) => Ok(()),
                    kad::QueryResult::StartProviding(Err(err))
                    | kad::QueryResult::RepublishProvider(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad start_providing failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutWorldHead { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutMembershipDirectory { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutPeerRecord { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetProviders {
            response,
            providers,
            error,
        } => {
            match result {
                kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                    providers: found,
                    ..
                })) => {
                    providers.extend(found);
                }
                kad::QueryResult::GetProviders(Ok(
                    kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetProviders(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_providers failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if !providers.is_empty() {
                    Ok(providers
                        .iter()
                        .map(|peer| ProviderRecord {
                            provider_id: peer.to_string(),
                            last_seen_ms: super::now_ms(),
                            storage_total_bytes: None,
                            storage_available_bytes: None,
                            uptime_ratio_per_mille: None,
                            challenge_pass_ratio_per_mille: None,
                            load_ratio_per_mille: None,
                            p50_read_latency_ms: None,
                        })
                        .collect())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(Vec::new())
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetWorldHead {
            response,
            head,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                    if let Ok(decoded) = super::decode_world_head(&record.record.value) {
                        *head = Some(decoded);
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(record) = records.first() {
                        if let Ok(decoded) = super::decode_world_head(&record.record.value) {
                            *head = Some(decoded);
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if head.is_some() {
                    Ok(head.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetMembershipDirectory {
            response,
            snapshot,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                    if let Ok(decoded) = super::decode_membership_directory(&record.record.value) {
                        *snapshot = Some(decoded);
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(record) = records.first() {
                        if let Ok(decoded) = super::decode_membership_directory(&record.record.value) {
                            *snapshot = Some(decoded);
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if snapshot.is_some() {
                    Ok(snapshot.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetPeerRecord {
            response,
            record,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(found))) => {
                    match decode_peer_record(&found.record.value) {
                        Ok(decoded) => *record = Some(decoded),
                        Err(err) => *error = Some(err),
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(found) = records.first() {
                        match decode_peer_record(&found.record.value) {
                            Ok(decoded) => *record = Some(decoded),
                            Err(err) => *error = Some(err),
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if record.is_some() {
                    Ok(record.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::DiscoverPeers { peers, error } => {
            match result {
                kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                    providers: found,
                    ..
                })) => {
                    peers.extend(found);
                }
                kad::QueryResult::GetProviders(Ok(
                    kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetProviders(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad discovery get_providers failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                if !peers.is_empty() {
                    return DhtProgressAction::DiscoverPeers(peers.iter().copied().collect());
                }
                if let Some(err) = error.take() {
                    return DhtProgressAction::DiscoveryError(err);
                }
            }
        }
        PendingDhtQuery::DiscoverPeerRecord {
            peer_id,
            record,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(found))) => {
                    match decode_peer_record(&found.record.value) {
                        Ok(decoded) => *record = Some(decoded),
                        Err(err) => *error = Some(err),
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(found) = records.first() {
                        match decode_peer_record(&found.record.value) {
                            Ok(decoded) => *record = Some(decoded),
                            Err(err) => *error = Some(err),
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let result = if record.is_some() {
                    Ok(record.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                return DhtProgressAction::DiscoverPeerRecord {
                    peer_id: *peer_id,
                    result,
                };
            }
        }
    }
    DhtProgressAction::None
}
