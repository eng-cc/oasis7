use super::*;

pub(crate) fn build_player_post_onboarding_snapshot(
    state: &ViewerState,
    control_feedback: Option<&WebTestApiControlFeedbackSnapshot>,
    locale: crate::i18n::UiLocale,
) -> PlayerPostOnboardingSnapshot {
    if let Some(gameplay) = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.player_gameplay.as_ref())
        .filter(|gameplay| gameplay.stage_id == PlayerGameplayStageId::PostOnboarding)
    {
        return build_player_post_onboarding_snapshot_from_gameplay(gameplay, locale);
    }

    build_player_post_onboarding_snapshot_from_events(state, control_feedback, locale)
}

fn build_player_post_onboarding_snapshot_from_events(
    state: &ViewerState,
    control_feedback: Option<&WebTestApiControlFeedbackSnapshot>,
    locale: crate::i18n::UiLocale,
) -> PlayerPostOnboardingSnapshot {
    let mut has_material_flow = false;
    let mut has_factory_ready = false;
    let mut has_recipe_running = false;
    let mut has_first_output = false;
    let mut latest_blocker = None::<(String, String)>;

    for event in &state.events {
        match &event.kind {
            WorldEventKind::RadiationHarvested { .. } | WorldEventKind::CompoundMined { .. } => {
                has_material_flow = true;
            }
            WorldEventKind::FactoryBuilt { .. } => {
                has_factory_ready = true;
            }
            WorldEventKind::RecipeScheduled { .. } => {
                has_recipe_running = true;
            }
            WorldEventKind::CompoundRefined { .. } => {
                has_material_flow = true;
                has_first_output = true;
            }
            WorldEventKind::RuntimeEvent { kind, domain_kind } => match kind.as_str() {
                "runtime.economy.factory_built" => {
                    has_factory_ready = true;
                }
                "runtime.economy.recipe_started" => {
                    has_recipe_running = true;
                }
                "runtime.economy.recipe_completed" => {
                    has_recipe_running = true;
                    has_first_output = true;
                }
                "runtime.economy.factory_production_blocked" => {
                    has_recipe_running = true;
                    let summary = domain_kind.as_deref().unwrap_or_default();
                    let reason = post_onboarding_summary_value(summary, "reason")
                        .unwrap_or("unknown")
                        .to_string();
                    let detail = post_onboarding_summary_value(summary, "detail")
                        .unwrap_or_default()
                        .to_string();
                    latest_blocker = Some((reason, detail));
                }
                "runtime.economy.factory_production_resumed" => {
                    has_recipe_running = true;
                    latest_blocker = None;
                }
                _ => {}
            },
            _ => {}
        }
    }

    let blocked_feedback = control_feedback.and_then(|feedback| {
        matches!(feedback.stage.as_str(), "blocked" | "completed_no_progress").then(|| {
            (
                feedback.reason.clone().unwrap_or_else(|| {
                    if locale.is_zh() {
                        "当前行动未形成有效推进".to_string()
                    } else {
                        "the latest command did not create useful forward progress".to_string()
                    }
                }),
                feedback.hint.clone().unwrap_or_default(),
            )
        })
    });

    if has_first_output {
        return PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::BranchReady,
            title: if locale.is_zh() {
                "下一阶段：选择中循环方向"
            } else {
                "Next Stage: Choose Your Mid-loop Path"
            },
            objective: if locale.is_zh() {
                "第一项持续工业能力已建立，开始把它扩张成稳定组织能力。".to_string()
            } else {
                "Your first sustainable industrial capability is online. Turn it into stable organizational momentum.".to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：已完成首个可见产出/稳定产线里程碑。".to_string()
            } else {
                "Stage progress: your first visible output or stable line milestone is complete."
                    .to_string()
            },
            progress_percent: 100,
            blocker_detail: None,
            next_step: if locale.is_zh() {
                "下一步：保持 Command 视图，继续扩产、推进治理提案，或为关键节点补防护。"
                    .to_string()
            } else {
                "Next: stay in Command view and either expand production, push governance, or secure a critical node."
                    .to_string()
            },
            branch_hint: Some(if locale.is_zh() {
                "已解锁分支：生产扩张 / 治理影响 / 冲突安全".to_string()
            } else {
                "Branches unlocked: Production Expansion / Governance Influence / Conflict Security"
                    .to_string()
            }),
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        };
    }

    if let Some((reason, detail)) = latest_blocker.or(blocked_feedback) {
        return PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::Blocked,
            title: if locale.is_zh() {
                "PostOnboarding：恢复持续能力"
            } else {
                "PostOnboarding: Recover Sustainable Capability"
            },
            objective: if locale.is_zh() {
                "优先恢复被阻塞的产线或能力链，而不是重复单次动作。".to_string()
            } else {
                "Recover the blocked line or capability chain instead of repeating one-off actions."
                    .to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：你已经进入经营阶段，但当前主线被阻塞。".to_string()
            } else {
                "Stage progress: you are in the management phase, but the primary line is blocked."
                    .to_string()
            },
            progress_percent: 68,
            blocker_detail: Some(post_onboarding_blocker_detail(
                reason.as_str(),
                detail.as_str(),
                locale,
            )),
            next_step: post_onboarding_blocker_next_step(reason.as_str(), detail.as_str(), locale),
            branch_hint: None,
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        };
    }

    if has_recipe_running {
        PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::Active,
            title: if locale.is_zh() {
                "PostOnboarding：稳定第一条产线"
            } else {
                "PostOnboarding: Stabilize Your First Line"
            },
            objective: if locale.is_zh() {
                "让第一条生产线连续推进，直到出现稳定产出或明确阻塞原因。".to_string()
            } else {
                "Keep your first production line moving until it produces stable output or exposes a clear blocker."
                    .to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：首条产线已启动，接下来重点看输出与停机原因。".to_string()
            } else {
                "Stage progress: the first line is running; now watch for output and stoppage reasons."
                    .to_string()
            },
            progress_percent: 72,
            blocker_detail: None,
            next_step: if locale.is_zh() {
                "下一步：保持 Command 视图，再推进 1~2 次，并观察是否出现产出、恢复或阻塞反馈。"
                    .to_string()
            } else {
                "Next: stay in Command view, advance 1-2 more times, and watch for output, recovery, or blocker feedback."
                    .to_string()
            },
            branch_hint: None,
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        }
    } else if has_factory_ready {
        PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::Active,
            title: if locale.is_zh() {
                "PostOnboarding：启动第一座工厂"
            } else {
                "PostOnboarding: Start Your First Factory Run"
            },
            objective: if locale.is_zh() {
                "把已建成的工厂推进成真正运转的持续能力。".to_string()
            } else {
                "Turn the factory you built into a running, repeatable capability.".to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：工厂已就绪，还差一次可见的生产推进。".to_string()
            } else {
                "Stage progress: the factory is ready; one visible production push remains."
                    .to_string()
            },
            progress_percent: 54,
            blocker_detail: None,
            next_step: if locale.is_zh() {
                "下一步：切到 Command 视图并继续推进，直到工厂启动配方、产出结果或返回阻塞原因。"
                    .to_string()
            } else {
                "Next: switch to Command view and keep advancing until the factory starts a recipe, yields output, or returns a blocker."
                    .to_string()
            },
            branch_hint: None,
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        }
    } else if has_material_flow {
        PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::Active,
            title: if locale.is_zh() {
                "PostOnboarding：把资源流变成产出"
            } else {
                "PostOnboarding: Turn Material Flow Into Output"
            },
            objective: if locale.is_zh() {
                "不要停留在一次性采集，继续把资源推进到可见产出。".to_string()
            } else {
                "Do not stop at one-off harvesting; push the resource flow into visible output."
                    .to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：基础资源已经动起来，接下来要形成第一项持续能力。".to_string()
            } else {
                "Stage progress: base resources are moving; now convert them into the first sustainable capability."
                    .to_string()
            },
            progress_percent: 38,
            blocker_detail: None,
            next_step: if locale.is_zh() {
                "下一步：继续在 Command 视图推进采集、精炼、建厂或首个配方，直到出现稳定产出。"
                    .to_string()
            } else {
                "Next: keep using Command view to harvest, refine, build, or start the first recipe until stable output appears."
                    .to_string()
            },
            branch_hint: None,
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        }
    } else {
        PlayerPostOnboardingSnapshot {
            status: PlayerPostOnboardingStatus::Active,
            title: if locale.is_zh() {
                "PostOnboarding：建立第一项持续能力"
            } else {
                "PostOnboarding: Establish Your First Sustainable Capability"
            },
            objective: if locale.is_zh() {
                "首局行动闭环已完成，下一步不是重复教程，而是做出第一项持续工业成果。".to_string()
            } else {
                "The first-session action loop is complete. The next step is not to repeat the tutorial, but to create your first sustainable industrial result."
                    .to_string()
            },
            progress_detail: if locale.is_zh() {
                "阶段进展：你已从“会操作”进入“会经营”的起点。".to_string()
            } else {
                "Stage progress: you have moved from 'can operate' into the start of 'can manage'."
                    .to_string()
            },
            progress_percent: 20,
            blocker_detail: None,
            next_step: if locale.is_zh() {
                "下一步：保持 Command 视图，再推进 2~3 次，优先追首个工业产出、首条稳定产线或一次明确的恢复反馈。"
                    .to_string()
            } else {
                "Next: stay in Command view and advance 2-3 more times, prioritizing the first industrial output, the first stable line, or one clear recovery signal."
                    .to_string()
            },
            branch_hint: None,
            action_label: if locale.is_zh() {
                "进入指挥并推进 1 步"
            } else {
                "Open command and advance 1 step"
            },
        }
    }
}

fn build_player_post_onboarding_snapshot_from_gameplay(
    gameplay: &PlayerGameplaySnapshot,
    locale: crate::i18n::UiLocale,
) -> PlayerPostOnboardingSnapshot {
    let status = match gameplay.stage_status {
        PlayerGameplayStageStatus::Active => PlayerPostOnboardingStatus::Active,
        PlayerGameplayStageStatus::Blocked => PlayerPostOnboardingStatus::Blocked,
        PlayerGameplayStageStatus::BranchReady => PlayerPostOnboardingStatus::BranchReady,
    };
    let blocker_reason = gameplay
        .blocker_kind
        .as_deref()
        .or(gameplay.blocker_detail.as_deref())
        .unwrap_or("unknown");
    let blocker_detail = matches!(status, PlayerPostOnboardingStatus::Blocked).then(|| {
        post_onboarding_blocker_detail(
            blocker_reason,
            gameplay.blocker_detail.as_deref().unwrap_or_default(),
            locale,
        )
    });
    let next_step = if matches!(status, PlayerPostOnboardingStatus::Blocked) {
        post_onboarding_blocker_next_step(
            blocker_reason,
            gameplay.blocker_detail.as_deref().unwrap_or_default(),
            locale,
        )
    } else if locale.is_zh() {
        localized_post_onboarding_next_step_for_goal(gameplay.goal_kind, locale)
    } else {
        gameplay.next_step_hint.clone()
    };
    let branch_hint = if locale.is_zh() {
        gameplay
            .branch_hint
            .as_ref()
            .map(|_| "已解锁分支：生产扩张 / 治理影响 / 冲突安全".to_string())
    } else {
        gameplay.branch_hint.clone()
    };

    PlayerPostOnboardingSnapshot {
        status,
        title: localized_post_onboarding_title_for_goal(gameplay.goal_kind, status, locale),
        objective: if locale.is_zh() {
            localized_post_onboarding_objective_for_goal(gameplay.goal_kind, status, locale)
        } else {
            gameplay.objective.clone()
        },
        progress_detail: if locale.is_zh() {
            localized_post_onboarding_progress_detail_for_goal(gameplay.goal_kind, status, locale)
        } else {
            gameplay.progress_detail.clone()
        },
        progress_percent: gameplay.progress_percent,
        blocker_detail,
        next_step,
        branch_hint,
        action_label: if locale.is_zh() {
            "进入指挥并推进 1 步"
        } else {
            "Open command and advance 1 step"
        },
    }
}
