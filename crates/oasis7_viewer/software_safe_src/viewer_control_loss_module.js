export const VIEWER_CONTROL_LOST_CODE = "control_lost";
export const VIEWER_CONTROL_LOST_NEXT_STEP = "reauthenticate_and_refresh_binding";
export const VIEWER_CONTROL_LOST_MESSAGE = "Agent control was lost; re-authenticate and refresh the current binding before retrying.";

function normalized(value) {
  return String(value || "").trim();
}

export function createViewerControlLossModule({ render, state }) {
  function isControlLossError(error) {
    const code = normalized(error?.code || error?.reason_code).toLowerCase();
    return code === "agent_control_forbidden" || code === VIEWER_CONTROL_LOST_CODE;
  }

  function isControlLost(agentId) {
    const id = normalized(agentId);
    return !!id && id === normalized(state.auth.controlLostAgentId);
  }

  function hiddenResponse(error = null) {
    return {
      status: "blocked",
      value_visibility: "hidden",
      code: VIEWER_CONTROL_LOST_CODE,
      reason_code: VIEWER_CONTROL_LOST_CODE,
      next_step: VIEWER_CONTROL_LOST_NEXT_STEP,
      message: VIEWER_CONTROL_LOST_MESSAGE,
      request_id: normalized(error?.request_id) || undefined,
    };
  }

  function markControlLost(agentId) {
    const id = normalized(agentId || state.auth.boundAgentId);
    if (!id) return hiddenResponse();
    state.auth.controlLostAgentId = id;
    state.auth.sessionEpoch = null;
    state.auth.bindingEpoch = null;
    state.auth.registrationStatus = "issued";
    state.auth.runtimeStatus = VIEWER_CONTROL_LOST_CODE;
    state.auth.recoveryErrorCode = VIEWER_CONTROL_LOST_CODE;
    state.auth.recoveryErrorMessage = VIEWER_CONTROL_LOST_MESSAGE;
    state.auth.pendingRequestedAgentId = id;
    render();
    return hiddenResponse();
  }

  function clearControlLost(agentId) {
    const marker = normalized(state.auth.controlLostAgentId);
    const id = normalized(agentId);
    if (marker && marker === id) {
      state.auth.controlLostAgentId = null;
      if (state.auth.recoveryErrorCode === VIEWER_CONTROL_LOST_CODE) {
        state.auth.recoveryErrorCode = null;
        state.auth.recoveryErrorMessage = null;
      }
    }
  }

  return { clearControlLost, hiddenResponse, isControlLost, isControlLossError, markControlLost };
}
