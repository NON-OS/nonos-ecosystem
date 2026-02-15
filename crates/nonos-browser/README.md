# nonos-browser

Browser session and security management.

## Features

- Tab lifecycle management
- Per-site isolation
- Security policy enforcement
- Session state tracking

## Modules

| Module | Purpose |
|--------|---------|
| `browser` | Core browser state |
| `tabs` | Tab creation and management |
| `security` | CSP, fingerprint protection |
| `ui` | UI helpers |

## Security Policies

- Content Security Policy enforcement
- Canvas fingerprint noise
- WebGL parameter spoofing
- Timing attack mitigation
- Per-site cookie isolation

## Usage

```rust
use nonos_browser::{Browser, Tab, SecurityPolicy};

let browser = Browser::new(SecurityPolicy::strict())?;
let tab = browser.new_tab("https://example.com")?;
```

## License

AGPL-3.0
