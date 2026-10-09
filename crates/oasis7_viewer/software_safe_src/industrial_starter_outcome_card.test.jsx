import { render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { IndustrialStarterOutcomeCard } from "./industrial_starter_outcome_card.jsx";
describe("visible starter outcome", () => {
 it("renders actual outcome and candidate without certifying stability or delivery", () => {
  const profile={profileId:"starter-industrial-smelter-to-assembler-v1",profileRevision:1,candidateAvailable:true,settled:{jobId:5,settledAt:27,owner:"starter-agent-0",acceptedBatches:12,consume:[{kind:"iron_ore",amount:48}],produce:[{kind:"iron_ingot",amount:36}],powerRequired:24}};
  const {container}=render(()=> <IndustrialStarterOutcomeCard profile={profile} locale={()=>"zh"} />);
  expect(container.textContent).toContain("iron_ingot × 36"); expect(container.textContent).toContain("iron_ore × 48");
  expect(container.textContent).toContain("用电需求: 24"); expect(container.textContent).toContain("production_only");
  expect(container.textContent).toContain("稳定运行与交付仍需分别验证"); expect(container.textContent).toContain("Assembler MK1");
 });
 it("shows unpublished legacy facts rather than estimates",()=>{const {container}=render(()=> <IndustrialStarterOutcomeCard profile={{profileId:"legacy",profileRevision:1,candidateAvailable:false,settled:null}} locale={()=>"zh"} />);expect(container.textContent).toContain("尚未发布");expect(container.textContent).not.toContain("已结算产出");});
});
