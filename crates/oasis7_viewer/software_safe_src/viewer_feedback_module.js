export function createViewerFeedbackModule({
  clone,
  feedbackBadgeClass,
  hostedActionPolicy,
  isLocaleZh,
  localeText,
  state,
}) {
  function snapshotControlFeedback(feedback) {
    if (!feedback) return null;
    return {
      id: feedback.id,
      action: feedback.action,
      accepted: feedback.accepted,
      stage: feedback.stage,
      reason: feedback.reason,
      hint: feedback.hint,
      effect: feedback.effect,
      deltaLogicalTime: feedback.deltaLogicalTime || 0,
      deltaEventSeq: feedback.deltaEventSeq || 0,
      deltaTraceCount: feedback.deltaTraceCount || 0,
    };
  }

  function snapshotSemanticFeedback(feedback) {
    if (!feedback) return null;
    return {
      id: feedback.id,
      kind: feedback.kind,
      action: feedback.action,
      agentId: feedback.agentId || null,
      accepted: feedback.accepted,
      stage: feedback.stage,
      ok: feedback.ok,
      reason: feedback.reason || null,
      effect: feedback.effect || null,
      response: clone(feedback.response) || null,
    };
  }

  function semanticFeedbackCode(feedback) {
    if (feedback?.stage !== "error") {
      return null;
    }
    const responseCode = String(feedback?.response?.code || "").trim();
    if (responseCode) {
      return responseCode;
    }
    const effectCode = String(feedback?.effect || "").trim();
    return effectCode || null;
  }

  function semanticFeedbackMessage(feedback) {
    const responseMessage = String(feedback?.response?.message || "").trim();
    if (responseMessage) {
      return responseMessage;
    }
    const reason = String(feedback?.reason || "").trim();
    return reason || null;
  }

  function formatPromptVersionLabel(value) {
    return `v${Math.max(0, Math.floor(Number(value || 0)))}`;
  }

  function humanizePromptField(field) {
    return String(field || "")
      .trim()
      .replaceAll("_", " ");
  }

  function summarizeAppliedFields(feedback) {
    const fields = Array.isArray(feedback?.response?.applied_fields)
      ? feedback.response.applied_fields
          .map(humanizePromptField)
          .filter(Boolean)
      : [];
    if (!fields.length) {
      return null;
    }
    return fields.join(", ");
  }

  function describeSemanticFeedback(feedback, locale = state.uiLocale) {
    if (!feedback) {
      return null;
    }
    const code = semanticFeedbackCode(feedback);
    const diagnostics = semanticFeedbackMessage(feedback);
    const description = {
      label: feedback.stage || "idle",
      summary: feedback.effect || diagnostics || (isLocaleZh(locale) ? "反馈已更新。" : "Feedback updated."),
      detail: null,
      code,
      diagnostics,
      badgeClass: feedbackBadgeClass(feedback),
    };

    if (feedback.stage === "error") {
      if (code === "llm_init_failed") {
        description.label = isLocaleZh(locale) ? "LLM 不可用" : "LLM unavailable";
        description.summary = isLocaleZh(locale)
          ? "当前栈没有可用的 LLM 配置，因此无法开始聊天。"
          : "Chat cannot start because this stack has no usable LLM configuration.";
        description.detail =
          isLocaleZh(locale)
            ? "请把 model、base URL 和 API key 写入当前 config.toml 或 OASIS7_LLM_* 环境变量，然后重启 launcher 栈。"
            : "Add model, base URL, and API key to the active config.toml or OASIS7_LLM_* env, then restart the launcher stack.";
        return description;
      }
      if (code === "target_version_not_found") {
        description.label = isLocaleZh(locale) ? "找不到回滚目标" : "Rollback target missing";
        description.summary = isLocaleZh(locale)
          ? "当前 Agent 没有这个可回滚版本。"
          : "The selected rollback version is not available for this agent.";
        description.detail = isLocaleZh(locale)
          ? "请先刷新 prompt 状态，或改选一个真实存在的保存版本后再重试。"
          : "Refresh prompt state or choose an existing saved version before retrying.";
        return description;
      }
      if (code === "rollback_noop") {
        description.label = isLocaleZh(locale) ? "回滚无变化" : "Rollback noop";
        description.summary = isLocaleZh(locale)
          ? "这个回滚目标不会改变当前 prompt。"
          : "That rollback target would not change the current prompt.";
        description.detail = isLocaleZh(locale)
          ? "只有在你确实要恢复不同 prompt 内容时，才需要选择更旧的版本。"
          : "Pick an older version only when you need to restore different prompt content.";
        return description;
      }
      if (feedback.kind === "prompt") {
        description.label = isLocaleZh(locale) ? "Prompt 失败" : "Prompt failed";
        description.summary = isLocaleZh(locale)
          ? "Prompt 控制没有完成。"
          : "Prompt control did not complete.";
        description.detail = isLocaleZh(locale)
          ? "展开诊断可查看后端拒绝的具体原因。"
          : "Open diagnostics for the exact backend rejection.";
        return description;
      }
      if (feedback.kind === "chat") {
        description.label = isLocaleZh(locale) ? "聊天失败" : "Chat failed";
        description.summary = isLocaleZh(locale)
          ? "Agent 聊天没有完成。"
          : "Agent chat did not complete.";
        description.detail = isLocaleZh(locale)
          ? "展开诊断可查看后端拒绝的具体原因。"
          : "Open diagnostics for the exact backend rejection.";
        return description;
      }
      if (feedback.kind === "gameplay_action") {
        description.label = isLocaleZh(locale) ? "玩法动作失败" : "Gameplay action failed";
        description.summary = isLocaleZh(locale)
          ? "正式玩法动作没有完成。"
          : "The gameplay action did not complete.";
        description.detail = isLocaleZh(locale)
          ? "展开诊断可查看 runtime 返回的拒绝原因。"
          : "Open diagnostics for the runtime rejection details.";
        return description;
      }
      description.label = code || "Request failed";
      description.summary = diagnostics || (isLocaleZh(locale) ? "请求失败。" : "The request failed.");
      description.detail = isLocaleZh(locale)
        ? "展开诊断可查看后端原始载荷。"
        : "Open diagnostics for the raw backend payload.";
      return description;
    }

    if (feedback.kind === "prompt") {
      const version = Number(feedback?.response?.version || 0);
      const appliedFields = summarizeAppliedFields(feedback);
      if (feedback.stage === "preview_ack") {
        description.label = isLocaleZh(locale) ? "预览已就绪" : "Preview ready";
        description.summary = isLocaleZh(locale)
          ? `Prompt 预览已基于 ${formatPromptVersionLabel(version)} 准备完成。`
          : `Prompt preview is ready from ${formatPromptVersionLabel(version)}.`;
        description.detail = isLocaleZh(locale)
          ? "应用前请先检查返回的摘要或 prompt 字段。"
          : "Review the returned digest or prompt fields before applying.";
        return description;
      }
      if (feedback.stage === "apply_ack") {
        description.label = isLocaleZh(locale) ? "Prompt 已保存" : "Prompt saved";
        description.summary = isLocaleZh(locale)
          ? `Prompt 改动已保存为 ${formatPromptVersionLabel(version)}。`
          : `Prompt changes are now saved as ${formatPromptVersionLabel(version)}.`;
        description.detail = appliedFields
          ? (isLocaleZh(locale) ? `已应用字段：${appliedFields}。` : `Applied fields: ${appliedFields}.`)
          : (isLocaleZh(locale) ? "Prompt 改动已被接受并持久化。" : "Prompt changes were accepted and persisted.");
        return description;
      }
      if (feedback.stage === "rollback_ack") {
        const restoredVersion = Number(feedback?.response?.rolled_back_to_version || 0);
        description.label = isLocaleZh(locale) ? "回滚已应用" : "Rollback applied";
        description.summary =
          isLocaleZh(locale)
            ? `当前生效 prompt 已保存为 ${formatPromptVersionLabel(version)}，其内容恢复自 ${formatPromptVersionLabel(restoredVersion)}。`
            : `Active prompt is now saved as ${formatPromptVersionLabel(version)} after restoring content from ${formatPromptVersionLabel(restoredVersion)}.`;
        description.detail =
          isLocaleZh(locale)
            ? "回滚会生成一个新的保存版本；下面输入框指向的是下一次回滚目标，不是刚刚恢复出来的版本。"
            : "Rollback creates a new saved version; the rollback input below points to the next target, not the version that was just restored.";
        return description;
      }
      description.label = isLocaleZh(locale) ? "Prompt 进行中" : "Prompt in progress";
      description.summary = feedback.effect || (isLocaleZh(locale) ? "Prompt 请求正在处理。" : "Prompt request is in flight.");
      description.detail = isLocaleZh(locale)
        ? "请等待 ack/error 返回后再发起下一次 prompt 操作。"
        : "Wait for ack/error before issuing another prompt action.";
      return description;
    }

    if (feedback.kind === "chat") {
      if (feedback.stage === "ack") {
        const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
        description.label = isLocaleZh(locale) ? "聊天已受理" : "Chat accepted";
        description.summary = isLocaleZh(locale)
          ? `消息已在 tick ${acceptedAtTick} 进入 runtime 队列。`
          : `Message entered the runtime queue at tick ${acceptedAtTick}.`;
        description.detail = isLocaleZh(locale)
          ? "请查看 Message Flow，确认玩家出站消息和后续 Agent 回应。"
          : "Watch Message Flow for the outbound player message and any inbound agent reply.";
        return description;
      }
      description.label = isLocaleZh(locale) ? "聊天进行中" : "Chat in progress";
      description.summary = feedback.effect || (isLocaleZh(locale) ? "聊天请求正在处理。" : "Chat request is in flight.");
      description.detail = isLocaleZh(locale)
        ? "请等待 ack/error 返回后再发送下一条消息。"
        : "Wait for ack/error before sending another message.";
      return description;
    }

    if (feedback.kind === "gameplay_action") {
      if (feedback.stage === "ack") {
        const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
        description.label = isLocaleZh(locale) ? "玩法动作已受理" : "Gameplay action accepted";
        description.summary = isLocaleZh(locale)
          ? `动作已在 tick ${acceptedAtTick} 进入 runtime 队列。`
          : `The action entered the runtime queue at tick ${acceptedAtTick}.`;
        description.detail = feedback?.response?.message
          || (isLocaleZh(locale)
            ? "请继续观察 gameplay feedback 或刷新后的快照。"
            : "Watch gameplay feedback or the refreshed snapshot for the next world-state change.");
        return description;
      }
      description.label = isLocaleZh(locale) ? "玩法动作进行中" : "Gameplay action in progress";
      description.summary = feedback.effect || (isLocaleZh(locale) ? "玩法动作请求正在处理。" : "Gameplay action request is in flight.");
      description.detail = isLocaleZh(locale)
        ? "请等待 ack/error 或新的 gameplay 快照反馈。"
        : "Wait for ack/error or a new gameplay snapshot update.";
      return description;
    }

    return description;
  }

  function describePromptVersionState(feedback = state.lastPromptFeedback, locale = state.uiLocale) {
    const currentVersion = Math.max(0, Math.floor(Number(state.promptDraft.currentVersion || 0)));
    const nextRollbackTargetVersion = Math.max(
      0,
      Math.floor(Number(state.promptDraft.rollbackTargetVersion || 0)),
    );
    const responseVersion = Number(feedback?.response?.version);
    const ackVersion = Number.isFinite(responseVersion) ? Math.max(0, Math.floor(responseVersion)) : currentVersion;
    const responseRollbackVersion = Number(feedback?.response?.rolled_back_to_version);
    const restoredFromVersion =
      feedback?.stage === "rollback_ack" && Number.isFinite(responseRollbackVersion)
        ? Math.max(0, Math.floor(responseRollbackVersion))
        : null;
    const summary = restoredFromVersion == null
      ? (isLocaleZh(locale)
          ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}。`
          : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}.`)
      : (isLocaleZh(locale)
          ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}；内容恢复自 ${formatPromptVersionLabel(restoredFromVersion)}。`
          : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}; content was restored from ${formatPromptVersionLabel(restoredFromVersion)}.`);
    const detail = restoredFromVersion == null
      ? (isLocaleZh(locale)
          ? `回滚输入框默认指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}。`
          : `The rollback input defaults to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}.`)
      : (isLocaleZh(locale)
          ? `这次回滚生成了新的保存版本 ${formatPromptVersionLabel(ackVersion)}。下面输入框现在指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}，不是刚恢复的版本。`
          : `The rollback created a new saved version ${formatPromptVersionLabel(ackVersion)}. The input below now points to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}, not the restored version.`);
    return {
      currentVersion,
      nextRollbackTargetVersion,
      ackVersion,
      restoredFromVersion,
      summary,
      detail,
    };
  }

  function buildGameplaySummary(locale = state.uiLocale) {
    const gameplay = state.snapshot?.player_gameplay;
    if (!gameplay || typeof gameplay !== "object") {
      return null;
    }

    const agents = Object.keys(state.snapshot?.model?.agents || {});
    const locations = Object.keys(state.snapshot?.model?.locations || {});
    const missingAgents = agents.length === 0;
    const missingLocations = locations.length === 0;
    const emptyEntityBlocker = missingAgents || missingLocations
      ? (() => {
        const missingLabel = missingAgents && missingLocations
          ? localeText(locale, "agents 与 locations", "agents and locations")
          : missingAgents
            ? "agents"
            : "locations";
        return {
          blockerKind: "runtime_snapshot_empty_entities",
          blockerDetail: localeText(
            locale,
            missingAgents && missingLocations
              ? "当前 gameplay 快照没有 Agent / 地点，说明 runtime world bootstrap 还没稳定输出正式实体。"
              : `当前 gameplay 快照缺少 ${missingLabel}，说明 runtime world bootstrap 还没稳定输出正式实体。`,
            missingAgents && missingLocations
              ? "The current gameplay snapshot has no agents/locations, so the runtime world bootstrap has not produced stable canonical entities yet."
              : `The current gameplay snapshot is missing ${missingLabel}, so the runtime world bootstrap has not produced stable canonical entities yet.`,
          ),
          nextStepHint: localeText(
            locale,
            "先刷新快照；如果实体仍为空，再去修复或重启 runtime world bootstrap 后再继续。",
            "Request a fresh snapshot first. If entities stay empty, repair or restart the runtime world bootstrap before continuing.",
          ),
          disabledReason: localeText(
            locale,
            `当前快照缺少 ${missingLabel}；刷新快照或修复 runtime bootstrap 后再试。`,
            `Current snapshot is missing ${missingLabel}; refresh the snapshot or repair runtime bootstrap before retrying.`,
          ),
        };
      })()
      : null;

    const progressRaw = Number(gameplay.progress_percent);
    const progressPercent = Number.isFinite(progressRaw)
      ? Math.max(0, Math.min(100, Math.floor(progressRaw)))
      : null;
    const acceptedIntentId = gameplay.accepted_intent_id || null;
    const intentSummary = gameplay.intent_summary || null;
    const intentScope = gameplay.intent_scope || null;
    const intentTarget = gameplay.intent_target || null;
    const statusReason = gameplay.status_reason || null;
    const lastWorldChange = gameplay.last_world_change || null;
    const resumeAnchor = gameplay.resume_anchor || null;
    const resumeNextStep = gameplay.resume_next_step || null;
    const availableActions = Array.isArray(gameplay.available_actions)
      ? gameplay.available_actions
        .map((action) => ({
          actionId: action?.action_id || null,
          label: action?.label || null,
          protocolAction: action?.protocol_action || null,
          targetAgentId: action?.target_agent_id || null,
          disabledReason:
            action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot"
              ? action?.disabled_reason || null
              : action?.disabled_reason || emptyEntityBlocker?.disabledReason || null,
          executeKind:
            action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot"
              ? "request_snapshot"
              : action?.protocol_action === "live_control.step"
                ? "step"
                : action?.protocol_action === "live_control.play"
                  ? "play"
                  : action?.protocol_action === "gameplay_action.submit"
                    ? "gameplay_action"
                    : action?.protocol_action === "agent_chat"
                      ? "agent_chat"
                      : "unsupported",
        }))
      : [];
    const recentFeedback = gameplay.recent_feedback && typeof gameplay.recent_feedback === "object"
      ? {
          action: gameplay.recent_feedback.action || null,
          stage: gameplay.recent_feedback.stage || null,
          effect: gameplay.recent_feedback.effect || null,
          reason: gameplay.recent_feedback.reason || null,
          hint: gameplay.recent_feedback.hint || null,
          deltaLogicalTime: Number(gameplay.recent_feedback.delta_logical_time || 0),
          deltaEventSeq: Number(gameplay.recent_feedback.delta_event_seq || 0),
        }
      : null;
    const runtimeBlockerKind = gameplay.blocker_kind || null;
    const runtimeBlockerDetail = gameplay.blocker_detail || null;
    const runtimeAlreadyPublishedEmptyEntityBlocker =
      runtimeBlockerKind === "runtime_snapshot_empty_entities";
    const resolvedStageStatus = emptyEntityBlocker ? "blocked" : gameplay.stage_status || null;
    const resolvedBlockerKind = runtimeAlreadyPublishedEmptyEntityBlocker
      ? runtimeBlockerKind
      : emptyEntityBlocker
        ? emptyEntityBlocker.blockerKind
        : runtimeBlockerKind;
    const resolvedBlockerDetail = runtimeAlreadyPublishedEmptyEntityBlocker
      ? runtimeBlockerDetail || emptyEntityBlocker?.blockerDetail || null
      : emptyEntityBlocker
        ? emptyEntityBlocker.blockerDetail
        : runtimeBlockerDetail;
    const executionState = emptyEntityBlocker
      ? "blocked"
      : gameplay.execution_state
      || (() => {
        const recentStage = String(recentFeedback?.stage || "").trim().toLowerCase();
        if (["accepted", "submitted", "queued", "ack"].includes(recentStage)) {
          return "accepted";
        }
        if (recentStage === "rejected") {
          return "rejected";
        }
        if (["blocked", "completed_no_progress"].includes(recentStage)) {
          return "blocked";
        }
        if (recentStage === "completed_advanced") {
          return "completed";
        }
        if (resolvedStageStatus === "blocked") {
          return "blocked";
        }
        if (resolvedStageStatus === "branch_ready") {
          return "completed";
        }
        return "executing";
      })();
    const executionStateLabel = (() => {
      switch (executionState) {
        case "accepted":
          return localeText(locale, "已接受", "Accepted");
        case "blocked":
          return localeText(locale, "已阻塞", "Blocked");
        case "completed":
          return localeText(locale, "已完成", "Completed");
        case "rejected":
          return localeText(locale, "已拒绝", "Rejected");
        default:
          return localeText(locale, "执行中", "Executing");
      }
    })();
    const executionStateMachine = [
      { id: "accepted", label: localeText(locale, "已接受", "Accepted") },
      { id: "executing", label: localeText(locale, "执行中", "Executing") },
      { id: "blocked", label: localeText(locale, "已阻塞", "Blocked") },
      { id: "completed", label: localeText(locale, "已完成", "Completed") },
      { id: "rejected", label: localeText(locale, "已拒绝", "Rejected") },
    ];
    const executionCauseKind = emptyEntityBlocker
      ? "world_constraint"
      : gameplay.causality_kind
      || (() => {
        if (executionState === "accepted") return "queued_for_execution";
        if (executionState === "rejected") return "request_rejected";
        if (executionState === "blocked") return "world_constraint";
        if (executionState === "completed") return "goal_progressed";
        return null;
      })();
    const executionCauseLabel = (() => {
      switch (executionCauseKind) {
        case "queued_for_execution":
          return localeText(locale, "等待执行", "Queued for Execution");
        case "world_constraint":
          return localeText(locale, "世界约束", "World Constraint");
        case "agent_override":
          return localeText(locale, "Agent 改走了别的允许路径", "Agent Chose Differently");
        case "goal_progressed":
          return localeText(locale, "世界已推进", "World Progressed");
        case "request_rejected":
          return localeText(locale, "请求被拒绝", "Request Rejected");
        default:
          return null;
      }
    })();
    const executionCauseDetail = emptyEntityBlocker
      ? resolvedBlockerDetail || emptyEntityBlocker.blockerDetail || null
      : gameplay.causality_detail
      || (() => {
        if (executionState === "blocked") {
          return resolvedBlockerDetail || recentFeedback?.reason || null;
        }
        if (executionState === "accepted") {
          return recentFeedback?.hint || recentFeedback?.effect || null;
        }
        if (executionState === "completed") {
          return recentFeedback?.effect || gameplay.progress_detail || null;
        }
        if (executionState === "rejected") {
          return recentFeedback?.reason || null;
        }
        return null;
      })();
    const executionSummary = (() => {
      if (executionCauseKind === "agent_override") {
        return localeText(
          locale,
          "本次目标已推动世界继续前进，但执行它的 Agent 最终采用了另一条被允许的计划。",
          "This goal still advanced the world, but the acting agent finished it through a different allowed plan.",
        );
      }
      switch (executionState) {
        case "accepted":
          return localeText(
            locale,
            "最新一条目标相关指令已经入队，正在等待 committed world delta 或后续回执。",
            "The latest goal-affecting command is queued and waiting for committed world delta or follow-up feedback.",
          );
        case "blocked":
          return localeText(
            locale,
            "当前目标没有继续推进，主要原因已经被归入可修复的 blocker taxonomy。",
            "The current goal is not moving forward; the primary reason is now grouped into a repairable blocker taxonomy.",
          );
        case "completed":
          return localeText(
            locale,
            "当前目标最近一次执行已经产生世界级结果，可以决定是继续放大、恢复，还是切到下一条主线。",
            "The current goal's latest execution already produced a world-level result; you can now amplify it, recover it, or pivot to the next line.",
          );
        case "rejected":
          return localeText(
            locale,
            "最新请求在执行前被拒绝，需要先修正请求本身或权限/模式前提。",
            "The latest request was rejected before execution; fix the request itself or its permission/mode prerequisites first.",
          );
        default:
          return localeText(
            locale,
            "当前目标正在执行中，先盯住状态机、主因果和下一步，再决定是否继续推进。",
            "The current goal is executing; read the state machine, primary causality, and next step before pushing again.",
          );
      }
    })();
    const blockerLabel = (() => {
      switch (resolvedBlockerKind) {
        case "material_shortage":
          return localeText(locale, "缺料", "Missing Material");
        case "power_shortage":
          return localeText(locale, "缺电", "Missing Power");
        case "governance_gate":
          return localeText(locale, "治理限制", "Governance Restriction");
        case "no_progress":
          return localeText(locale, "没有前进", "No Forward Progress");
        case "llm_required":
          return localeText(locale, "缺少玩法能力", "Missing Gameplay Capability");
        case "runtime_sync_unavailable":
          return localeText(locale, "运行时同步不可用", "Runtime Sync Unavailable");
        case "execution_world_not_ready":
          return localeText(locale, "执行世界未就绪", "Execution World Not Ready");
        case "runtime_snapshot_empty_entities":
          return localeText(locale, "空快照", "Empty Snapshot");
        default:
          return resolvedBlockerKind || null;
      }
    })();
    const recommendedAction = availableActions
      .filter((action) => !action.disabledReason)
      .sort((left, right) => {
        const priority = (action) => {
          switch (action.executeKind) {
            case "gameplay_action":
              return 0;
            case "step":
              return 1;
            case "play":
              return 2;
            case "request_snapshot":
              return 3;
            case "agent_chat":
              return 4;
            default:
              return 5;
          }
        };
        return priority(left) - priority(right);
      })[0] || null;
    const acceptedIntentSummary = intentSummary
      || acceptedIntentId
      || localeText(
        locale,
        "还没有一条被正式接受的玩家意图",
        "No player-facing accepted intent yet",
      );
    const acceptedIntentDetail = (() => {
      if (lastWorldChange) {
        return lastWorldChange;
      }
      if (statusReason) {
        return statusReason;
      }
      if (recentFeedback?.hint) {
        return recentFeedback.hint;
      }
      return localeText(
        locale,
        "先提交一个玩法动作，再看系统如何确认、推进或阻塞它。",
        "Submit one gameplay action first, then read how the system confirms, advances, or blocks it.",
      );
    })();
    const narrativeNextStep = emptyEntityBlocker
      ? emptyEntityBlocker.nextStepHint
      : gameplay.next_step_hint || resumeNextStep || null;
    const narrativeBlockerDetail = resolvedBlockerDetail || statusReason || recentFeedback?.reason || null;

    return {
      stageId: gameplay.stage_id || null,
      stageStatus: resolvedStageStatus,
      acceptedIntentId,
      acceptedIntentSummary,
      acceptedIntentScope: intentScope,
      acceptedIntentTarget: intentTarget,
      acceptedIntentDetail,
      statusReason,
      lastWorldChange,
      resumeAnchor,
      resumeNextStep,
      executionState,
      executionStateLabel,
      executionStateMachine,
      executionSummary,
      executionCauseKind,
      executionCauseLabel,
      executionCauseDetail,
      goalId: gameplay.goal_id || null,
      goalKind: gameplay.goal_kind || null,
      goalTitle: gameplay.goal_title || null,
      objective: gameplay.objective || null,
      progressDetail: gameplay.progress_detail || null,
      progressPercent,
      blockerKind: resolvedBlockerKind,
      blockerLabel,
      blockerDetail: resolvedBlockerDetail,
      blockerSupplementalDetail: emptyEntityBlocker && runtimeBlockerDetail && !runtimeAlreadyPublishedEmptyEntityBlocker
        ? runtimeBlockerDetail
        : null,
      nextStepHint: runtimeAlreadyPublishedEmptyEntityBlocker
        ? gameplay.next_step_hint || emptyEntityBlocker?.nextStepHint || resumeNextStep || null
        : emptyEntityBlocker
          ? emptyEntityBlocker.nextStepHint
          : gameplay.next_step_hint || resumeNextStep || null,
      branchHint: gameplay.branch_hint || null,
      narrativeBlockerDetail,
      narrativeNextStep,
      entityCounts: {
        agents: agents.length,
        locations: locations.length,
      },
      availableActions,
      recommendedAction,
      recentFeedback,
      agentClaim: clone(gameplay.agent_claim),
      assetGovernanceHandoff: isLocaleZh(locale)
        ? "资产 / 治理动作仍在单独 lane 处理；viewer 这里不会直接暴露主代币转账表单。"
        : "Asset/governance actions remain a separate lane. viewer exposes no main token transfer form here.",
    };
  }

  return {
    buildGameplaySummary,
    describePromptVersionState,
    describeSemanticFeedback,
    snapshotControlFeedback,
    snapshotSemanticFeedback,
  };
}
