# Authentication Patterns

Login flows, session persistence, OAuth, 2FA, and authenticated browsing.

Use the owned-session setup and cleanup from [session-management.md](session-management.md)
in the same shell before these examples (also inside any standalone script):

```bash
AB_SESSION="$(agent-browser session id --scope worktree --prefix auth)" || exit $?
: "${AB_SESSION:?Session creation returned an empty id}"
ab() { agent-browser --session "$AB_SESSION" "$@"; }
```

Install the scoped `EXIT INT TERM` cleanup from that reference after setup.
Every `ab` invocation passes the owned id explicitly; an ambient
`AGENT_BROWSER_SESSION` alone is not sufficient. Never use the default session
or global multi-session cleanup. This follows the repository's 0.37.1 session
contract; if the installed CLI differs, read `agent-browser skills get core --full`
before adapting these examples.

Prefer `--restore --restore-save auto` plus a restore check for new persistent-login flows. Explicit state save/load below is a compatibility recipe for exchanging a concrete state file.

**Related**: [session-management.md](session-management.md) for state persistence details, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [Basic Login Flow](#basic-login-flow)
- [Saving Authentication State](#saving-authentication-state)
- [Restoring Authentication](#restoring-authentication)
- [OAuth / SSO Flows](#oauth--sso-flows)
- [Two-Factor Authentication](#two-factor-authentication)
- [HTTP Basic Auth](#http-basic-auth)
- [Cookie-Based Auth](#cookie-based-auth)
- [Token Refresh Handling](#token-refresh-handling)
- [Security Best Practices](#security-best-practices)

## Basic Login Flow

```bash
# Navigate to login page
ab open https://app.example.com/login
ab wait --load networkidle

# Get form elements
ab snapshot -i
# Output: @e1 [input type="email"], @e2 [input type="password"], @e3 [button] "Sign In"

# Fill credentials
ab fill @e1 "user@example.com"
ab fill @e2 "password123"

# Submit
ab click @e3
ab wait --load networkidle

# Verify login succeeded
ab get url  # Should be dashboard, not login
```

## Saving Authentication State

First prepare a new private directory outside every checkout. The example below
uses the user's home directory; verify its resolved location is outside the repo
and that local ACLs and backup/sync policy are suitable before saving tokens.
Repo-external placement reduces accidental commits; it is not a secure-storage
guarantee. Keep state files private and never upload them as test evidence.

```bash
umask 077
AUTH_STATE_DIR="$(mktemp -d "$HOME/.oasis7-browser-auth.XXXXXX")" || exit $?
export AUTH_STATE_DIR
```

Use the same directory for later load examples; do not generate a new directory
when restoring an existing state. After logging in, save state for reuse:

```bash
# Login first (see above)
ab open https://app.example.com/login
ab snapshot -i
ab fill @e1 "user@example.com"
ab fill @e2 "password123"
ab click @e3
ab wait --url "**/dashboard"

# Save authenticated state
ab state save "$AUTH_STATE_DIR/auth-state.json"
```

## Restoring Authentication

Skip login by loading saved state:

```bash
# Load saved auth state
ab state load "$AUTH_STATE_DIR/auth-state.json"

# Navigate directly to protected page
ab open https://app.example.com/dashboard

# Verify authenticated
ab snapshot -i
```

## OAuth / SSO Flows

For OAuth redirects:

```bash
# Start OAuth flow
ab open https://app.example.com/auth/google

# Handle redirects automatically
ab wait --url "**/accounts.google.com**"
ab snapshot -i

# Fill Google credentials
ab fill @e1 "user@gmail.com"
ab click @e2  # Next button
ab wait 2000
ab snapshot -i
ab fill @e3 "password"
ab click @e4  # Sign in

# Wait for redirect back
ab wait --url "**/app.example.com**"
ab state save "$AUTH_STATE_DIR/oauth-state.json"
```

## Two-Factor Authentication

Handle 2FA with manual intervention:

```bash
# Login with credentials
ab open https://app.example.com/login --headed  # Show browser
ab snapshot -i
ab fill @e1 "user@example.com"
ab fill @e2 "password123"
ab click @e3

# Wait for user to complete 2FA manually
echo "Complete 2FA in the browser window..."
ab wait --url "**/dashboard" --timeout 120000

# Save state after 2FA
ab state save "$AUTH_STATE_DIR/2fa-state.json"
```

## HTTP Basic Auth

For sites using HTTP Basic Authentication:

```bash
# Set credentials before navigation
ab set credentials username password

# Navigate to protected resource
ab open https://protected.example.com/api
```

## Cookie-Based Auth

Manually set authentication cookies:

```bash
# Set auth cookie
ab cookies set session_token "abc123xyz"

# Navigate to protected page
ab open https://app.example.com/dashboard
```

## Token Refresh Handling

For sessions with expiring tokens:

```bash
#!/bin/bash
# Wrapper that handles token refresh

: "${AUTH_STATE_DIR:?Prepare the private repo-external state directory first}"
STATE_FILE="$AUTH_STATE_DIR/auth-state.json"

# Try loading existing state
if [[ -f "$STATE_FILE" ]]; then
    ab state load "$STATE_FILE"
    ab open https://app.example.com/dashboard

    # Check if session is still valid
    URL=$(ab get url)
    if [[ "$URL" == *"/login"* ]]; then
        echo "Session expired, re-authenticating..."
        # Perform fresh login
        ab snapshot -i
        ab fill @e1 "$USERNAME"
        ab fill @e2 "$PASSWORD"
        ab click @e3
        ab wait --url "**/dashboard"
        ab state save "$STATE_FILE"
    fi
else
    # First-time login
    ab open https://app.example.com/login
    # ... login flow ...
fi
```

## Security Best Practices

1. **Keep state files outside the repo** - They contain session tokens.
   Use the private `AUTH_STATE_DIR` above. If legacy state exists in a checkout,
   stop and assess it rather than relying on ignore rules: `.gitignore` does not
   remove tracked files or revoke exposed tokens. This example is not evidence
   that any real token has been exposed.

2. **Use environment variables for credentials**
   ```bash
   ab fill @e1 "$APP_USERNAME"
   ab fill @e2 "$APP_PASSWORD"
   ```

3. **Clean up after automation**
   ```bash
   ab cookies clear
   rm -f "$AUTH_STATE_DIR/auth-state.json"
   ```

4. **Use short-lived sessions for CI/CD**
   ```bash
   # Don't persist state in CI
   ab open https://app.example.com/login
   # ... login and perform actions ...
   ab close  # Session ends, nothing persisted
   ```
