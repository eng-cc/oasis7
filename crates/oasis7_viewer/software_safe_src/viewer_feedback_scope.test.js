import { describe,it,expect } from "vitest";
import { createMutable } from "solid-js/store";
import { bindFeedbackScope,feedbackForScope } from "./viewer_feedback_scope.js";
describe("feedback display scope",()=>{
  it("keeps pending operation identity but masks its private text after scope change and late reassignment",()=>{
    const state=createMutable({worldId:"W",auth:{playerId:"A"},feedback:null});
    const original=bindFeedbackScope({agentId:"A",effect:"private A",operation:"original"},state);
    state.feedback=original;expect(feedbackForScope(state.feedback,state)).not.toBeNull();
    state.auth.playerId="B";expect(feedbackForScope(state.feedback,state)).toBeNull();
    original.effect="late A result";state.feedback=original;
    expect(feedbackForScope(state.feedback,state)).toBeNull();
    expect(state.feedback.operation).toBe("original");
    state.feedback=bindFeedbackScope({agentId:"B",effect:"B result"},state);
    expect(feedbackForScope(state.feedback,state).effect).toBe("B result");
  });
});
