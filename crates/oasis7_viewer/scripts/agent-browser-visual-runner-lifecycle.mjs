import { spawnSync } from "node:child_process";

const SIGNAL_EXIT_CODES = {
  SIGINT: 130,
  SIGTERM: 143,
};

/**
 * Own the lifecycle of one agent-browser session for a visual runner.
 *
 * The pre-open cleanup is a separate phase from terminal cleanup: a runner
 * may deliberately reopen its page during a visual pass. Terminal cleanup is
 * idempotent so a signal handler and the process exit hook cannot issue a
 * second close for the same browser context.
 */
export function createOwnedSessionLifecycle({
  command,
  prefixArgs = [],
  session,
  timeoutMs = 10_000,
  processLike = process,
  spawnSyncImpl = spawnSync,
}) {
  if (!command) throw new Error("agent-browser command is required");
  if (!session) throw new Error("agent-browser session is required");

  let closeAttempted = false;
  const closeArgs = [...prefixArgs, "--session", session, "close"];

  const close = () => {
    if (closeAttempted) return false;
    closeAttempted = true;
    try {
      spawnSyncImpl(command, closeArgs, { stdio: "ignore", timeout: timeoutMs });
    } catch {
      // Cleanup must not hide the original runner failure or signal.
    }
    return true;
  };

  const prepare = () => {
    close();
    // A subsequent open starts a new owned browser context in this runner.
    closeAttempted = false;
  };

  const onSignal = (signal) => {
    close();
    processLike.exit(SIGNAL_EXIT_CODES[signal] || 1);
  };

  processLike.once("SIGINT", () => onSignal("SIGINT"));
  processLike.once("SIGTERM", () => onSignal("SIGTERM"));
  processLike.once("exit", close);

  return { close, prepare };
}
