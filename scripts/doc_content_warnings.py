"""Only explicitly classified document findings may use this advisory exit path."""

def report_findings(findings: list[str], warn_only: bool) -> int:
    hard = False
    for finding in findings:
        security = "escapes repository" in finding or "escapes the repository" in finding or "symlink-not-allowed" in finding
        hard = hard or security
        if warn_only and not security:
            # GitHub workflow-command escaping also prevents annotation injection.
            escaped = finding.replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
            print(f'::warning::{escaped}')
        else:
            print(finding)
    if warn_only:
        print(f'document-content: WARN ({len(findings)} finding(s); advisory, not proof of conformance)')
        return 1 if hard else 0
    return 1
