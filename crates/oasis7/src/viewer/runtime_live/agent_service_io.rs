//! Immutable transport capsules. Execution never borrows Viewer state.
use crate::world_service::{
    client::{RemoteWorldServiceClient, WorldServicePort},
    verified_view::VerifiedWorldView,
    wire::WorldServicePayloadV1,
};
use oasis7_client_api::world_service::*;
use serde_json::Value;

pub(super) enum AgentServiceProgress {
    Idle,
    Advanced,
    NeedsIo(AgentServiceIoJob),
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct AgentServiceIoToken {
    pub(super) generation: u64,
    pub(super) config_digest: String,
    pub(super) phase_digest: String,
}
pub(super) enum AgentServiceIoOperation {
    Submit(SubmitIntentRequest<WorldServicePayloadV1>),
    Lookup {
        request: LookupIntentRequest,
        original: WorldServicePayloadV1,
    },
    View(ReadWorldViewRequest),
    Metadata(crate::simulator::ProviderLoopbackHttpClient),
}
pub(super) struct AgentServiceIoJob {
    pub(super) token: AgentServiceIoToken,
    pub(super) client: Option<RemoteWorldServiceClient>,
    pub(super) operation: AgentServiceIoOperation,
}
pub(super) enum AgentServiceIoResponse {
    Submit(SubmitObservation<Value>),
    Intent(IntentResponse<Value>),
    View(VerifiedWorldView),
    Metadata(
        crate::simulator::ProviderInfo,
        crate::simulator::ProviderHealth,
    ),
}
pub(super) struct AgentServiceIoResult {
    pub(super) token: AgentServiceIoToken,
    pub(super) response: Result<AgentServiceIoResponse, String>,
}
impl AgentServiceIoJob {
    pub(super) fn execute(self) -> AgentServiceIoResult {
        let response = match self.operation {
            AgentServiceIoOperation::Metadata(client) => client
                .provider_info()
                .and_then(|info| client.provider_health().map(|health| (info, health)))
                .map(|(info, health)| AgentServiceIoResponse::Metadata(info, health))
                .map_err(|error| error.to_string()),
            operation => match self.client {
                None => Err("canonical service transport configuration missing".into()),
                Some(client) => match operation {
                    AgentServiceIoOperation::Submit(request) => {
                        client.submit(request).map(AgentServiceIoResponse::Submit)
                    }
                    AgentServiceIoOperation::Lookup { request, original } => client
                        .lookup(request, original)
                        .map(AgentServiceIoResponse::Intent),
                    AgentServiceIoOperation::View(request) => {
                        client.read_view(request).map(AgentServiceIoResponse::View)
                    }
                    AgentServiceIoOperation::Metadata(_) => unreachable!(),
                }
                .map_err(|error| error.to_string()),
            },
        };
        AgentServiceIoResult {
            token: self.token,
            response,
        }
    }
}
