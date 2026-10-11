// Generated from Rust DTOs by cargo run -p oasis7-command-center-core --example export_types
export type Project = { id: string, path: string, gitCommonDir: string, branch: string | null, head: string, environment: string, };
export type SourceStatus = { kind: string, state: string, lastAttempt: string | null, lastSuccess: string | null, sourceRevision: string | null, generation: string, coverage: string, error: string | null, };
export type DocumentSection = { id: string, path: string, title: string, content: string, line: number, revision: string, committedRevision: string, dirty: boolean, factKind: string, };
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
