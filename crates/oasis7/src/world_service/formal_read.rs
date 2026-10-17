//! World projection specialization of the independent client proof consumer.
pub use oasis7_game_client::formal_read::{
    FormalNormalDurableSetReadStepV1, FormalReadCheckpointV2, FormalReadFencingEvidenceV1,
    FormalReadHistoryPageV1, FormalReadHistoryV1, FormalReadOriginQualificationV2,
    FormalReadProjection, FormalReadReplayCursorV1, FormalReadStepV1, FormalRecoveryReadStepV1,
    MAX_FORMAL_HISTORY_STEPS,
};
pub type FormalReadVerifier =
    oasis7_game_client::formal_read::FormalReadVerifier<super::projection::WorldServiceProjection>;
pub type VerifiedFormalProjectionV1 = oasis7_game_client::formal_read::VerifiedFormalProjectionV1<
    super::projection::WorldServiceProjection,
>;

impl oasis7_game_client::formal_read::FormalReadProjection
    for super::projection::WorldServiceProjection
{
    fn validate_contents(
        response: &oasis7_client_api::world_service::ReadWorldViewResponse<Self>,
        request: &oasis7_client_api::world_service::ReadWorldViewRequest,
    ) -> Result<(), String> {
        super::verified_view::VerifiedWorldView::validate_contents(response, request)
    }
}

#[cfg(test)]
#[path = "formal_read_adapter_tests.rs"]
mod tests;
