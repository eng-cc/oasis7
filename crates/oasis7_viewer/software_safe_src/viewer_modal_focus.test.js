import { expect, it } from "vitest";
import { keepFocusInDialog } from "./viewer_modal_focus.js";

it("keeps late renderer focus in the account modal and releases it on cleanup", () => {
  document.body.innerHTML = '<canvas tabindex="0"></canvas><div role="dialog" tabindex="-1"><button>Sign in</button></div>';
  const dialog = document.querySelector('[role="dialog"]');
  const canvas = document.querySelector("canvas");
  const release = keepFocusInDialog(() => dialog);
  canvas.focus();
  expect(document.activeElement).toBe(dialog.querySelector("button"));
  release();
  canvas.focus();
  expect(document.activeElement).toBe(canvas);
  document.body.innerHTML = "";
});

it("does not redirect focus to a detached dialog", () => {
  const dialog = document.createElement("div");
  document.body.innerHTML = '<button>Continue</button>';
  const release = keepFocusInDialog(() => dialog);
  document.querySelector("button").focus();
  expect(document.activeElement).toBe(document.querySelector("button"));
  release();
  document.body.innerHTML = "";
});
