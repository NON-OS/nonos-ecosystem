# NONOS Ecosystem Beta Release

Build, verify, and run the complete ecosystem.

## System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| RAM | 8 GB | 16 GB (ZK proofs) |
| Disk | 2 GB | 10 GB |
| Rust | 1.75+ | Latest stable |
| Node.js | 18+ | 20+ |
| OS | Linux, macOS, Windows | Linux, macOS |

### Platform Setup

**macOS**
```bash
xcode-select --install
brew install node
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Linux (Ubuntu/Debian)**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev \
    libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

# Node via nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows**
1. Install Visual Studio Build Tools 2019+
2. Install WebView2 Runtime
3. Install Rust via rustup-init.exe
4. Install Node.js from nodejs.org

## Build

### Full Ecosystem
```bash
git clone https://github.com/NON-OS/nonos-ecosystem.git
cd nonos-ecosystem
cargo build --release
```

Binaries output:
- `target/release/nonos` - daemon
- `target/release/nonos-dash` - terminal UI

### Browser Application
```bash
cd ui
npm install
npm run tauri build
```

Output: `ui/src-tauri/target/release/nonos-app`

### Contracts (Optional)
```bash
cd contracts
forge build
forge test
```

## First Run

### Initialize Node
```bash
./target/release/nonos init \
    --nickname "mynode" \
    --reward-address "0x..."
```

Creates:
- `~/.nonos/config.toml` - configuration
- `~/.nonos/p2p_identity.key` - node identity
- `~/.nonos/identity/` - ZK identity storage

### Start Daemon
```bash
./target/release/nonos run
```

Verify running:
```bash
./target/release/nonos status
```

### Start Browser
```bash
# Development
cd ui && npm run tauri dev

# Or run built app
./ui/src-tauri/target/release/nonos-app
```

## Verification Checklist

Run these before shipping:

### 1. Build Clean
```bash
cargo build --release 2>&1 | grep -E "^error" && echo "FAIL" || echo "PASS"
```

### 2. Tests Pass
```bash
cargo test --workspace
```

Expected: All tests pass, including:
- 52 crypto tests (Poseidon, Groth16, encryption)
- Wallet tests (HD derivation, stealth addresses)
- ZK proof tests (membership, nullifiers)

### 3. ZK Proofs Work
```bash
cargo test -p nonos-crypto zk_proofs -- --nocapture
```

Should show:
- Proof generation completes
- Verification passes
- Tampered proof rejected
- Wrong root rejected

### 4. Wallet Encryption Correct
```bash
cargo test -p nonos-wallet -- --nocapture
```

Verify:
- Argon2id parameters: 64 MiB, 3 iterations
- AES-256-GCM encryption
- Wrong password rejected

### 5. DNS Leak Prevention
```bash
cargo test -p nonos-anyone proxy -- --nocapture
```

Verify SOCKS5h mode (remote DNS).

### 6. Localhost Binding
```bash
./target/release/nonos run &
sleep 2
netstat -an | grep -E "8420|9050" | grep -v "127.0.0.1" && echo "FAIL: exposed" || echo "PASS: localhost only"
```

### 7. Secret Redaction
```bash
cargo test -p nonos-types redaction -- --nocapture
```

All secrets show `[REDACTED]` in debug output.

## Configuration

Default config at `~/.nonos/config.toml`:

```toml
[node]
nickname = "anon"
reward_address = ""

[p2p]
listen_port = 9432
bootstrap_peers = [
    "/ip4/102.211.56.24/tcp/9432/p2p/12D3KooW..."
]
max_peers = 50

[api]
bind_address = "127.0.0.1"
port = 8420
auth_enabled = true

[privacy]
tracking_blocker = true
cache_mixing = true
zk_sessions = true

[logging]
level = "info"
```

## Systemd Setup (Linux)

Generate service file:
```bash
./target/release/nonos systemd --output-dir /etc/systemd/system --user $USER
```

Install:
```bash
sudo systemctl daemon-reload
sudo systemctl enable nonos
sudo systemctl start nonos
sudo systemctl status nonos
```

## Diagnostics

```bash
# Basic status
nonos status

# Health check
nonos check

# Extended diagnostics
nonos check --full

# Peer connections
nonos peers list

# Network stats
nonos stats

# Terminal dashboard
nonos dash
```

## Troubleshooting

**Daemon won't start**
```bash
# Check for existing instance
pgrep -f "nonos run" && echo "Already running"

# Check port availability
netstat -an | grep 9432

# Check logs
tail -100 ~/.nonos/logs/daemon.log
```

**Can't connect to peers**
```bash
# Verify network
curl -s https://api.ipify.org && echo " - Network OK"

# Check firewall
sudo ufw status | grep 9432

# Test bootstrap
nonos check
```

**ZK proof generation slow**
First proof generates proving/verifying keys (~30-60 seconds). Subsequent proofs use cached keys (~5-15 seconds). This is expected.

**Browser not loading pages**
```bash
# Verify daemon running
nonos status

# Check proxy port
curl --socks5-hostname 127.0.0.1:9050 https://check.torproject.org
```

## Release Checklist

| Item | Command | Expected |
|------|---------|----------|
| Build succeeds | `cargo build --release` | No errors |
| Tests pass | `cargo test --workspace` | All pass |
| ZK proofs | `cargo test zk_proofs` | Generation + verification |
| Encryption | `cargo test wallet` | Argon2id + AES-GCM |
| DNS safety | `cargo test proxy` | SOCKS5h mode |
| Localhost only | `netstat` check | 127.0.0.1 binding |
| Secrets redacted | `cargo test redaction` | [REDACTED] output |
| Node starts | `nonos run` | Status OK |
| Peers connect | `nonos peers list` | >0 peers |
| Browser works | Load page | Through Anyone Network |

## Support

- Issues: https://github.com/NON-OS/nonos-ecosystem/issues
- Docs: https://docs.nonos.systems
- Security: team@nonos.systems
