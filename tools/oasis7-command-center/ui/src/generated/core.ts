// Generated from Rust DTOs by cargo run -p oasis7-command-center-core --example export_types
export type ContextBinding = { id: string, bindingVersion: number, root: string, storeId: string, projectId: string, registeredProjectId: string, name: string, directoryIdentity: string, };
export type ContextEntry = { id: string, revision: string, rawText: string, metadataJson: string, verificationState: string, };
export type ContextSummary = { id: string, title: string, kind: string, environment: string, state: string, revision: string, verification_state: string, summary: string, };
export type ContextQuery = { entries: Array<ContextSummary>, complete: boolean, total_matches: number | null, has_more: boolean, next_cursor: string | null, warnings: Array<string>, read_at: string, };
export type ContextSaveRequest = { bindingId: string, bindingVersion: number, id: string, rawText: string,
/**
 * None means create only, never unconditional overwrite.
 */
expectedRevision: string | null, actor: string, };
export type ToolObservation = { tool_id: string, state: string, path: string | null, reason: string | null, observed_at: string, info: unknown, };
export type FileSignInput = { job_id: string, file_path: string, signer_id: string, grant_id: string, context: unknown, };
export type FileSignPrepared = { job_id: string, prepared_fingerprint: string, file_sha256: string, payload_sha256: string, operation_key: string, work_dir: string, result: unknown, };
export type ToolJob = { id: string, fingerprint: string, resource: string, kind: string, state: string, observed_at: string, result: string | null, };
export type CliInstallation = { tool_id: string, release_id: string, path: string, sha256: string, terminal_path_configured: boolean, };
export type PrivateInputRef = { bindingId: string, bindingVersion: number, entryId: string, revision: string, scope: string, };
export type PrivateInputPreview = { source: PrivateInputRef, text: string, };
export type Project = { id: string, path: string, gitCommonDir: string, branch: string | null, head: string, environment: string, };
export type SourceStatus = { kind: string, state: string, lastAttempt: string | null, lastSuccess: string | null, sourceRevision: string | null, generation: string, coverage: string, error: string | null, };
export type DocumentLifecycle = { status: string | null, successorDeclaration: string | null, successorTargets: Array<string>, successorReason: string | null, issue: string | null, };
export type DocumentSection = { id: string, path: string, title: string, content: string, line: number, revision: string, committedRevision: string, dirty: boolean, factKind: string, lifecycle: DocumentLifecycle | null, };
export type Preferences = { projectPath: string | null, view: string, query: string, };
export type DecisionDraft = { id: string | null, content: string, reason: string, scope: string, relatedSection: string | null, confirmed: boolean, };
export type DecisionRecord = { id: string, content: string, reason: string, scope: string, relatedSection: string | null, state: string, confirmedAt: string | null, formalUpdate: string, implementationSync: string, };
export type ReadingPosition = { sectionId: string, revision: string, line: number, };
export type GitSnapshot = { status: string, worktrees: string, };
export type WorkAssociation = { workUrl: string, goalSectionIds: Array<string>, confirmedAt: string, };
export type Snapshot = { associations: Array<WorkAssociation>, project: Project | null, sources: Array<SourceStatus>, sections: Array<DocumentSection>, decisions: Array<DecisionRecord>, preferences: Preferences, readingPositions: Array<ReadingPosition>, git: GitSnapshot, };
export type GitHubSnapshot = { state: string, repository: string | null, account: string | null, observedAt: string | null, lastAttempt: string | null, coverage: string, error: string | null, pulls: Array<PullRequest>, };
export type PullRequest = { id: string, title: string, url: string, state: string, headOid: string, reviewDecision: string | null, checks: Array<Check>, };
export type Check = { name: string, state: string, testedOid: string | null, url: string | null, };
export type RuntimeTarget = { instanceId: string, threadId: string, turnId: string, };
export type RuntimeSession = { purpose: string, scope: string, model: string | null, basisRevision: string, threadId: string, instanceId: string, turnId: string | null, state: string, owned: boolean, backgroundState: string, restorableHistory: boolean, privateSources: Array<PrivateInputRef>, };
export type RuntimeOperation = { id: string, instanceId: string, threadId: string | null, turnId: string | null, method: string, state: string, fingerprint: string, createdAt: string, error: string | null, };
export type RuntimeQuestion = { id: string, header: string, question: string, options: Array<string>, isSecret: boolean, };
export type RuntimeRequest = { requestId: string, instanceId: string, threadId: string, turnId: string, method: string, state: string, summary: string, questions: Array<RuntimeQuestion>, allowedDecisions: Array<string>, };
export type RequestAnswer = { requestId: string, target: RuntimeTarget, decision: string | null, answers: { [key in string]?: Array<string> }, };
export type RuntimeSnapshot = { projectId: string, environment: string, enabled: boolean, experimental: boolean, state: string, version: string | null, instanceId: string | null, capabilities: Array<string>, coverage: string, error: string | null, sessions: Array<RuntimeSession>, requests: Array<RuntimeRequest>, operations: Array<RuntimeOperation>, logs: Array<string>, logsTruncated: boolean, exitWaiting: boolean, };
export type QuitStatus = { safe: boolean, active: number, unknown: number, pendingRequests: number, waiting: boolean, reason: string, };
export type SessionConfig = { purpose: string, scope: string, model: string | null, basisRevision: string, };
