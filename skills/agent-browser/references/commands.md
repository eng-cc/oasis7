# Command Reference

Complete reference for all agent-browser commands. For quick start and common patterns, see SKILL.md.

Use the owned-session setup and cleanup from [session-management.md](session-management.md)
in the same shell before these examples (also inside any standalone script):

```bash
AB_SESSION="$(agent-browser session id --scope worktree --prefix commands)" || exit $?
: "${AB_SESSION:?Session creation returned an empty id}"
ab() { agent-browser --session "$AB_SESSION" "$@"; }
```

Install the scoped `EXIT INT TERM` cleanup from that reference after setup.
Every `ab` invocation passes the owned id explicitly; an ambient
`AGENT_BROWSER_SESSION` alone is not sufficient. Never use the default session
or global multi-session cleanup. This follows the repository's 0.37.1 session
contract; if the installed CLI differs, read `agent-browser skills get core --full`
before adapting these examples.

## Navigation

```bash
ab open <url>      # Navigate to URL (aliases: goto, navigate)
                              # Supports: https://, http://, file://, about:, data://
                              # Auto-prepends https:// if no protocol given
ab back            # Go back
ab forward         # Go forward
ab reload          # Reload page
ab close           # Close browser (aliases: quit, exit)
ab connect 9222    # Connect to browser via CDP port
```

## Snapshot (page analysis)

```bash
ab snapshot            # Full accessibility tree
ab snapshot -i         # Interactive elements only (recommended)
ab snapshot -c         # Compact output
ab snapshot -d 3       # Limit depth to 3
ab snapshot -s "#main" # Scope to CSS selector
```

## Interactions (use @refs from snapshot)

```bash
ab click @e1           # Click
ab click @e1 --new-tab # Click and open in new tab
ab dblclick @e1        # Double-click
ab focus @e1           # Focus element
ab fill @e2 "text"     # Clear and type
ab type @e2 "text"     # Type without clearing
ab press Enter         # Press key (alias: key)
ab press Control+a     # Key combination
ab keydown Shift       # Hold key down
ab keyup Shift         # Release key
ab hover @e1           # Hover
ab check @e1           # Check checkbox
ab uncheck @e1         # Uncheck checkbox
ab select @e1 "value"  # Select dropdown option
ab select @e1 "a" "b"  # Select multiple options
ab scroll down 500     # Scroll page (default: down 300px)
ab scrollintoview @e1  # Scroll element into view (alias: scrollinto)
ab drag @e1 @e2        # Drag and drop
ab upload @e1 file.pdf # Upload files
```

## Get Information

```bash
ab get text @e1        # Get element text
ab get html @e1        # Get innerHTML
ab get value @e1       # Get input value
ab get attr @e1 href   # Get attribute
ab get title           # Get page title
ab get url             # Get current URL
ab get count ".item"   # Count matching elements
ab get box @e1         # Get bounding box
ab get styles @e1      # Get computed styles (font, color, bg, etc.)
```

## Check State

```bash
ab is visible @e1      # Check if visible
ab is enabled @e1      # Check if enabled
ab is checked @e1      # Check if checked
```

## Screenshots and PDF

```bash
ab screenshot          # Save to temporary directory
ab screenshot path.png # Save to specific path
ab screenshot --full   # Full page
ab pdf output.pdf      # Save as PDF
```

## Video Recording

```bash
ab record start ./demo.webm    # Start recording
ab click @e1                   # Perform actions
ab record stop                 # Stop and save video
ab record restart ./take2.webm # Stop current + start new
```

## Wait

```bash
ab wait @e1                     # Wait for element
ab wait 2000                    # Wait milliseconds
ab wait --text "Success"        # Wait for text (or -t)
ab wait --url "**/dashboard"    # Wait for URL pattern (or -u)
ab wait --load networkidle      # Wait for network idle (or -l)
ab wait --fn "window.ready"     # Wait for JS condition (or -f)
```

## Mouse Control

```bash
ab mouse move 100 200      # Move mouse
ab mouse down left         # Press button
ab mouse up left           # Release button
ab mouse wheel 100         # Scroll wheel
```

## Semantic Locators (alternative to refs)

```bash
ab find role button click --name "Submit"
ab find text "Sign In" click
ab find text "Sign In" click --exact      # Exact match only
ab find label "Email" fill "user@test.com"
ab find placeholder "Search" type "query"
ab find alt "Logo" click
ab find title "Close" click
ab find testid "submit-btn" click
ab find first ".item" click
ab find last ".item" click
ab find nth 2 "a" hover
```

## Browser Settings

```bash
ab set viewport 1920 1080          # Set viewport size
ab set viewport 1920 1080 2        # 2x retina (same CSS size, higher res screenshots)
ab set device "iPhone 14"          # Emulate device
ab set geo 37.7749 -122.4194       # Set geolocation (alias: geolocation)
ab set offline on                  # Toggle offline mode
ab set headers '{"X-Key":"v"}'     # Extra HTTP headers
ab set credentials user pass       # HTTP basic auth (alias: auth)
ab set media dark                  # Emulate color scheme
ab set media light reduced-motion  # Light mode + reduced motion
```

## Cookies and Storage

```bash
ab cookies                     # Get all cookies
ab cookies set name value      # Set cookie
ab cookies clear               # Clear cookies
ab storage local               # Get all localStorage
ab storage local key           # Get specific key
ab storage local set k v       # Set value
ab storage local clear         # Clear all
```

## Network

```bash
ab network route <url>              # Intercept requests
ab network route <url> --abort      # Block requests
ab network route <url> --body '{}'  # Mock response
ab network unroute [url]            # Remove routes
ab network requests                 # View tracked requests
ab network requests --filter api    # Filter requests
```

## Tabs and Windows

```bash
ab tab                 # List tabs
ab tab new [url]       # New tab
ab tab 2               # Switch to tab by index
ab tab close           # Close current tab
ab tab close 2         # Close tab by index
ab window new          # New window
```

## Frames

```bash
ab frame "#iframe"     # Switch to iframe
ab frame main          # Back to main frame
```

## Dialogs

```bash
ab dialog accept [text]  # Accept dialog
ab dialog dismiss        # Dismiss dialog
```

## JavaScript

```bash
ab eval "document.title"          # Simple expressions only
ab eval -b "<base64>"             # Any JavaScript (base64 encoded)
ab eval --stdin                   # Read script from stdin
```

Use `-b`/`--base64` or `--stdin` for reliable execution. Shell escaping with nested quotes and special characters is error-prone.

```bash
# Base64 encode your script, then:
ab eval -b "ZG9jdW1lbnQucXVlcnlTZWxlY3RvcignW3NyYyo9Il9uZXh0Il0nKQ=="

# Or use stdin with heredoc for multiline scripts:
cat <<'EOF' | ab eval --stdin
const links = document.querySelectorAll('a');
Array.from(links).map(a => a.href);
EOF
```

## State Management

First prepare the private repo-external `AUTH_STATE_DIR` as described in
[authentication.md](authentication.md#saving-authentication-state).

```bash
ab state save "$AUTH_STATE_DIR/auth-state.json" # Save cookies, storage, auth state
ab state load "$AUTH_STATE_DIR/auth-state.json" # Restore saved state
```

## Global Options

```bash
agent-browser --session "$AB_SESSION" ... # Explicit owned browser session
ab --json ...              # JSON output for parsing
ab --headed ...            # Show browser window (not headless)
ab --full ...              # Full page screenshot (-f)
ab --cdp <port> ...        # Connect via Chrome DevTools Protocol
ab -p <provider> ...       # Cloud browser provider (--provider)
ab --proxy <url> ...       # Use proxy server
ab --proxy-bypass <hosts>  # Hosts to bypass proxy
ab --headers <json> ...    # HTTP headers scoped to URL's origin
ab --executable-path <p>   # Custom browser executable
ab --extension <path> ...  # Load browser extension (repeatable)
ab --ignore-https-errors   # Ignore SSL certificate errors
agent-browser --help                  # Show help (-h)
agent-browser --version               # Show version (-V)
ab <command> --help        # Show detailed help for a command
```

## Debugging

```bash
ab --headed open example.com   # Show browser window
ab --cdp 9222 snapshot         # Connect via CDP port
ab connect 9222                # Alternative: connect command
ab console                     # View console messages
ab console --clear             # Clear console
ab errors                      # View page errors
ab errors --clear              # Clear errors
ab highlight @e1               # Highlight element
ab trace start                 # Start recording trace
ab trace stop trace.zip        # Stop and save trace
ab profiler start              # Start Chrome DevTools profiling
ab profiler stop trace.json    # Stop and save profile
```

## Environment Variables

```bash
AGENT_BROWSER_SESSION="$AB_SESSION"         # Ambient hint; still use explicit ab wrapper
AGENT_BROWSER_EXECUTABLE_PATH="/path/chrome" # Custom browser path
AGENT_BROWSER_EXTENSIONS="/ext1,/ext2"       # Comma-separated extension paths
AGENT_BROWSER_PROVIDER="browserbase"         # Cloud browser provider
AGENT_BROWSER_STREAM_PORT="9223"             # WebSocket streaming port
AGENT_BROWSER_HOME="/path/to/agent-browser"  # Custom install location
```
