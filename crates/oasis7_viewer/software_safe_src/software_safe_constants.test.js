import { describe, expect, it } from "vitest";
import {
  HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
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
});
