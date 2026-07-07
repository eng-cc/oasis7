import { describe, expect, it } from "vitest";
import {
  HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
  LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE,
  isHostedPublicJoinDeploymentMode,
} from "./software_safe_constants.js";

describe("software safe constants", () => {
  it("centralizes hosted public join deployment-mode matching", () => {
    expect(isHostedPublicJoinDeploymentMode(HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE)).toBe(true);
    expect(isHostedPublicJoinDeploymentMode(` ${HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE} `)).toBe(true);

    expect(isHostedPublicJoinDeploymentMode("trusted_local_only")).toBe(false);
    expect(isHostedPublicJoinDeploymentMode("hosted-public-join")).toBe(false);
    expect(isHostedPublicJoinDeploymentMode(null)).toBe(false);
    expect(isHostedPublicJoinDeploymentMode(undefined)).toBe(false);
  });

  it("centralizes the legacy viewer auth bootstrap source id", () => {
    expect(LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE).toBe("legacy_viewer_auth_bootstrap");
  });
});
