export function createViewerAuthSurfaceModule({
  getSearchParams,
  localeText,
  selectedAgentInteractionMode,
  state,
  windowRef,
}) {
  function resolveHostedAccessHint() {
    const raw = getSearchParams().get("hosted_access");
    if (!raw) {
      return null;
    }
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === "object" ? parsed : null;
    } catch (_) {
      return null;
    }
  }

  function hostnameFromUrl(raw) {
    const value = String(raw || "").trim();
    if (!value) return null;
    try {
      return new URL(value, windowRef.location.href).hostname || null;
    } catch (_) {
      return null;
    }
  }

  function isLoopbackHostname(raw) {
    const value = String(raw || "").trim().toLowerCase();
    return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]";
  }

  function authDeploymentHint(auth) {
    const hostedMode = String(state.hostedAccess?.deployment_mode || "").trim();
    if (hostedMode === "hosted_public_join") {
      if (auth.available && auth.source === "legacy_viewer_auth_bootstrap") {
        return "hosted_public_join_contract_with_legacy_bootstrap";
      }
      return auth.available
        ? "hosted_public_join_contract_with_browser_session"
        : "hosted_public_join_contract";
    }
    if (hostedMode === "trusted_local_only") {
      return auth.available ? "trusted_local_contract" : "trusted_local_contract_guest";
    }
    const params = getSearchParams();
    const wsHost = hostnameFromUrl(state.wsUrl || params.get("ws") || params.get("addr") || "");
    const pageHost = String(windowRef.location.hostname || "").trim();
    const remoteOriginLikely = [pageHost, wsHost].filter(Boolean).some((host) => !isLoopbackHostname(host));
    if (auth.available) {
      return remoteOriginLikely ? "remote_origin_legacy_bootstrap" : "trusted_local_preview";
    }
    return remoteOriginLikely ? "hosted_public_join_likely" : "guest_only_or_missing_bootstrap";
  }

  function isHostedPublicJoinHint(deploymentHint) {
    return [
      "hosted_public_join_contract",
      "hosted_public_join_contract_with_browser_session",
      "hosted_public_join_contract_with_legacy_bootstrap",
      "hosted_public_join_likely",
    ].includes(deploymentHint);
  }

  function hostedActionPolicy(actionId) {
    const normalizedActionId = actionId === "prompt_control"
      ? "prompt_control_apply"
      : actionId;
    return state.hostedAccess?.action_matrix?.find((policy) => policy?.action_id === normalizedActionId) || null;
  }

  function guestSessionReason(auth, deploymentHint) {
    if (auth.available) {
      return auth.source === "legacy_viewer_auth_bootstrap"
        ? "guest session has already been superseded by the current preview player auth lane"
        : "guest session has already been superseded by a hosted-issued player identity";
    }
    if (isHostedPublicJoinHint(deploymentHint)) {
      return auth.error || "this browser is still guest-only; hosted public join must issue a player identity before low-risk interaction unlocks";
    }
    return auth.error || "viewer auth bootstrap is unavailable, so the browser cannot leave guest session";
  }

  function playerSessionReason(auth, deploymentHint) {
    if (auth.available) {
      if (auth.source === "legacy_viewer_auth_bootstrap") {
        return "player interaction is currently unlocked through legacy viewer auth bootstrap in trusted preview mode";
      }
      if (auth.registrationStatus === "registered") {
        return "player interaction is unlocked through hosted-issued player_id + browser-local ephemeral Ed25519 session";
      }
      if (auth.registrationStatus === "registering" || auth.registrationStatus === "issued") {
        return "browser-local hosted identity is ready; runtime player-session registration is still in progress";
      }
      return auth.error || "hosted player identity exists, but runtime registration still needs recovery";
    }
    if (isHostedPublicJoinHint(deploymentHint)) {
      return auth.error || "player session upgrade/login is still pending hosted issue";
    }
    return auth.error || "viewer auth bootstrap is missing or incomplete";
  }

  function strongAuthReason() {
    return "strong auth remains a separate upgrade plane; viewer already supports hosted player-session issue/reconnect/release, but backend reauth stays preview-only for prompt_control and still does not unlock hosted-ready asset/governance proofs";
  }

  function buildStrongAuthTier() {
    const promptPolicy = hostedActionPolicy("prompt_control");
    if (!promptPolicy || promptPolicy.required_auth !== "strong_auth") {
      return {
        status: "separate_upgrade_plane",
        reason: strongAuthReason(),
      };
    }
    if (promptPolicy.availability === "public_player_plane_with_backend_reauth_preview") {
      if (!state.auth.available) {
        return {
          status: "upgrade_after_player_session",
          reason:
            "hosted preview backend reauth is available on this join lane after the browser acquires a player_session",
        };
      }
      if (state.auth.registrationStatus === "registered") {
        return {
          status: "preview_backend_reauth_available",
          reason:
            "hosted preview backend reauth is available after the browser-local player_session has completed runtime registration for prompt_control",
        };
      }
      return {
        status: "issued_pending_register",
        reason:
          "hosted preview backend reauth stays pending until the browser-local player_session finishes runtime registration",
      };
    }
    if (promptPolicy.availability === "trusted_local_preview_only") {
      return {
        status: state.auth.available ? "active_legacy_preview" : "trusted_local_only",
        reason:
          "trusted_local_preview keeps prompt_control on the legacy local preview lane; hosted/public strong_auth still remains outside this window",
      };
    }
    return {
      status: "blocked_until_strong_auth",
      reason: promptPolicy.reason || strongAuthReason(),
    };
  }

  function isStrongAuthSensitiveAction(actionId) {
    const policy = hostedActionPolicy(actionId);
    if (policy) {
      return policy.required_auth === "strong_auth";
    }
    return actionId === "prompt_control" || actionId === "main_token_transfer";
  }

  function buildSemanticCapability(actionId) {
    const observerOnly = selectedAgentInteractionMode() === "observer_only";
    const deploymentHint = authDeploymentHint(state.auth);
    const strongAuthSensitive = isStrongAuthSensitiveAction(actionId);
    const policy = hostedActionPolicy(actionId);
    if (observerOnly) {
      return {
        actionId,
        enabled: false,
        code: "observer_only",
        reason:
          "selected agent runs through the provider-backed loopback bridge; viewer stays observer-only for prompt/chat on this lane",
      };
    }
    if (policy) {
      if (policy.required_auth === "strong_auth") {
        const isLocalPreviewOnly = policy.availability === "trusted_local_preview_only";
        const isBackendGrantPreview = policy.availability === "public_player_plane_with_backend_reauth_preview";
        if (isLocalPreviewOnly && state.auth.available && !isHostedPublicJoinHint(deploymentHint)) {
          return {
            actionId,
            enabled: true,
            code: null,
            reason: policy.reason || "trusted local preview currently allows this strong-auth-marked action through preview bootstrap",
          };
        }
        if (isBackendGrantPreview && state.auth.available) {
          return {
            actionId,
            enabled: true,
            code: null,
            reason: policy.reason || `${actionId} is available through browser-local player auth plus backend re-authorization`,
          };
        }
        if (isBackendGrantPreview && !state.auth.available) {
          return {
            actionId,
            enabled: false,
            code: "auth_level_insufficient",
            reason: `${actionId} requires player_session before backend re-authorization can upgrade it to strong_auth`,
          };
        }
        return {
          actionId,
          enabled: false,
          code: "strong_auth_required",
          reason: policy.reason || strongAuthReason(),
        };
      }
      if (!state.auth.available) {
        return {
          actionId,
          enabled: false,
          code: "auth_level_insufficient",
          reason: `${actionId} requires ${policy.required_auth}; current browser remains guest_session only`,
        };
      }
      return {
        actionId,
        enabled: true,
        code: null,
        reason: policy.reason || `${actionId} is allowed on the ${policy.required_auth} lane`,
      };
    }
    if (strongAuthSensitive && isHostedPublicJoinHint(deploymentHint)) {
      const hostedStrongAuthReason = state.auth.available
        ? `${actionId} still requires strong_auth on the hosted public join path; this browser only has a legacy preview player_session, so backend re-authorization or a private operator plane must take over`
        : `${actionId} requires strong_auth on the hosted public join path; acquire a player_session first, then complete the hosted re-authorization step for this action`;
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: hostedStrongAuthReason,
      };
    }
    if (strongAuthSensitive && state.auth.available && deploymentHint === "remote_origin_legacy_bootstrap") {
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: `${actionId} is blocked on remote-origin legacy bootstrap; hosted/public prompt control must move to strong_auth or private operator plane`,
      };
    }
    if (!state.auth.available) {
      const reason = isHostedPublicJoinHint(deploymentHint)
        ? `${actionId} requires player_session; this browser is still guest_session only on the hosted public join path`
        : `${actionId} requires viewer auth bootstrap; current status: ${state.auth.error || "missing"}`;
      return {
        actionId,
        enabled: false,
        code: "auth_level_insufficient",
        reason,
      };
    }
    return {
      actionId,
      enabled: true,
      code: null,
      reason: strongAuthSensitive
        ? "prompt_control stays enabled only in trusted_local_preview via legacy viewer auth bootstrap; hosted/public strong_auth remains pending"
        : "player_session is active via legacy viewer auth bootstrap preview",
    };
  }

  function buildAuthSurfaceModel() {
    const deploymentHint = authDeploymentHint(state.auth);
    const promptCapability = buildSemanticCapability("prompt_control");
    const chatCapability = buildSemanticCapability("agent_chat");
    const mainTokenTransferCapability = buildSemanticCapability("main_token_transfer");
    const strongAuthTier = buildStrongAuthTier();
    const currentTier = state.auth.available ? "player_session" : "guest_session";
    const source = state.hostedAccess
      ? state.auth.available
        ? state.auth.source === "legacy_viewer_auth_bootstrap"
          ? "legacy_viewer_auth_bootstrap+hosted_access_hint"
          : "hosted_player_issue+browser_local_ephemeral_key"
        : "hosted_access_hint"
      : state.auth.available
        ? state.auth.source
        : "guest_only";
    return {
      deploymentHint,
      source,
      currentTier,
      currentTierReason:
        currentTier === "player_session"
          ? playerSessionReason(state.auth, deploymentHint)
          : guestSessionReason(state.auth, deploymentHint),
      tiers: [
        {
          id: "guest_session",
          label: "guest_session",
          status: state.auth.available ? "superseded" : "active",
          reason: guestSessionReason(state.auth, deploymentHint),
        },
        {
          id: "player_session",
          label: "player_session",
          status: state.auth.available
            ? state.auth.source === "legacy_viewer_auth_bootstrap"
              ? "active_legacy_preview"
              : state.auth.registrationStatus === "registered"
                ? "active_hosted_session"
                : "issued_pending_register"
            : "not_issued",
          reason: playerSessionReason(state.auth, deploymentHint),
        },
        {
          id: "strong_auth",
          label: "strong_auth",
          status: strongAuthTier.status,
          reason: strongAuthTier.reason,
        },
      ],
      capabilities: {
        prompt_control: promptCapability,
        agent_chat: chatCapability,
        main_token_transfer: mainTokenTransferCapability,
        strong_auth_actions: mainTokenTransferCapability,
      },
      reconnect: state.auth.available
        ? state.auth.source === "legacy_viewer_auth_bootstrap"
          ? "reconnect still depends on the current preview bootstrap; hosted player-session reconnect/release is available only after switching away from legacy bootstrap"
          : state.auth.registrationStatus === "registered"
            ? "page reload will reuse the browser-local hosted key and attempt reconnect_sync first"
            : "browser-local hosted key is persisted, but runtime session restore is still pending this page load"
        : isHostedPublicJoinHint(deploymentHint)
          ? buildHostedRecoveryHint("en")?.detail
            || "hosted public join recovers by acquiring a player_session first, then re-registering it through reconnect_sync"
          : "page reload is possible once viewer auth bootstrap or hosted player-session issue succeeds",
    };
  }

  function buildHostedActionMatrixView() {
    const matrix = Array.isArray(state.hostedAccess?.action_matrix)
      ? state.hostedAccess.action_matrix
      : [];
    return matrix.map((policy) => {
      const actionId = String(policy?.action_id || "").trim();
      const capability = buildSemanticCapability(actionId);
      return {
        actionId,
        requiredAuth: String(policy?.required_auth || "").trim() || "unknown",
        availability: String(policy?.availability || "").trim() || "unknown",
        reason: String(policy?.reason || capability.reason || "").trim(),
        enabled: capability.enabled === true,
        code: capability.code || null,
        capabilityReason: capability.reason || null,
      };
    });
  }

  function buildHostedRecoveryHint(locale = state.uiLocale) {
    if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
      return null;
    }
    if (state.auth.available) {
      return null;
    }
    const errorText = String(state.auth.error || "").trim();
    const revokeReason = String(state.auth.revokeReason || "").trim();
    const revokedBy = String(state.auth.revokedBy || "").trim();
    if (!errorText) {
      return null;
    }
    if (errorText.includes("released locally")) {
      return {
        kind: "released",
        title: localeText(locale, "当前浏览器已主动释放会话", "This browser already released its session"),
        detail: localeText(
          locale,
          "当前 player_session 已在本地释放。重新申请一个新的 hosted player session 后，viewer 会再做 reconnect_sync。",
          "The current player_session was released locally. Acquire a fresh hosted player session and viewer will attempt reconnect_sync again.",
        ),
        cta: localeText(locale, "重新申请 Hosted Player Session", "Re-acquire Hosted Player Session"),
      };
    }
    if (revokeReason === "agent_already_bound" || errorText.includes("agent is already bound")) {
      return {
        kind: "agent_already_bound",
        title: localeText(locale, "所选 Agent 已绑定其他玩家", "Selected agent is already bound"),
        detail: localeText(
          locale,
          "当前 agent 已被其他 player_session 占用。请换一个未占用 Agent，或等待原会话释放后再重试。",
          "The selected agent is already owned by another player_session. Choose an unbound agent or wait for the previous session to release it.",
        ),
        cta: localeText(locale, "重新申请并改绑 Agent", "Retry with a different agent"),
      };
    }
    if (revokeReason === "runtime_registration_failed") {
      return {
        kind: "runtime_registration_failed",
        title: localeText(locale, "Runtime 注册没有完成", "Runtime registration did not finish"),
        detail: localeText(
          locale,
          "浏览器已经拿到 hosted identity，但 runtime register/reconnect 失败。请先重试 issue/reconnect，再检查 launcher/runtime 日志。",
          "The browser already received a hosted identity, but runtime register/reconnect failed. Retry issue/reconnect first, then inspect launcher/runtime logs.",
        ),
        cta: localeText(locale, "重试注册 / reconnect", "Retry register / reconnect"),
      };
    }
    if (revokedBy) {
      return {
        kind: "revoked",
        title: localeText(locale, "当前会话已被回收", "The current session was revoked"),
        detail: localeText(
          locale,
          `运行时或操作员 ${revokedBy} 已回收当前浏览器会话，原因是 ${revokeReason || errorText || "unknown"}。需要重新申请一个新的 Hosted Player Session，玩法、聊天和 prompt 才能继续。`,
          `The runtime or operator revoked this browser session by ${revokedBy}. Reason: ${revokeReason || errorText || "unknown"}. You need to acquire a fresh hosted player session before gameplay, chat, or prompt actions can continue.`,
        ),
        cta: localeText(locale, "重新申请 Hosted Player Session", "Re-acquire Hosted Player Session"),
      };
    }
    return {
      kind: "issue_required",
      title: localeText(locale, "当前浏览器还没有 player_session", "This browser still has no player_session"),
      detail: localeText(
        locale,
        "当前 hosted public join 需要先 issue player_session，再让 runtime 完成 register/reconnect。",
        "Hosted public join must issue a player_session first, then let runtime finish register/reconnect.",
      ),
      cta: localeText(locale, "申请 Hosted Player Session", "Acquire Hosted Player Session"),
    };
  }

  return {
    authDeploymentHint,
    buildAuthSurfaceModel,
    buildHostedActionMatrixView,
    buildHostedRecoveryHint,
    buildSemanticCapability,
    hostedActionPolicy,
    resolveHostedAccessHint,
  };
}
