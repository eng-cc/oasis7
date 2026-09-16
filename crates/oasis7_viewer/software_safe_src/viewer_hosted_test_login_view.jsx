import { Show } from "solid-js";

function shouldShowHostedTestLogin() {
  const value = String(new URLSearchParams(window.location.search || "").get("hosted_test_login") || "")
    .trim()
    .toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}

export function HostedTestLoginOptIn(props) {
  const locale = () => props.locale?.() ?? "en";
  const tr = props.tr;
  return (
    <Show when={shouldShowHostedTestLogin()}>
      <div class="stack" data-viewer-fixture-state="hosted_test_login_opt_in">
        <div class="toolbar">
          <button
            type="button"
            data-auth-action="test-login"
            disabled={props.core.state.hostedLogin.startInFlight || props.core.state.auth.issueInFlight}
            onClick={() => {
              void props.core.startHostedTestLogin();
            }}
          >
            {tr(locale(), "免邮箱测试登录", "Email-free Test Login")}
          </button>
        </div>
        <div class="feedback-detail">
          {tr(
            locale(),
            "仅用于显式开启测试入口的托管环境；身份与玩家会话仍由后端签发。",
            "For an explicitly opted-in hosted test lane; identity and player session still come from the backend.",
          )}
        </div>
      </div>
    </Show>
  );
}
