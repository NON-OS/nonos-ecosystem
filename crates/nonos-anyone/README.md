# nonos-anyone

Anyone Protocol client for anonymous network routing.

## Overview

Manages the Anyone Protocol binary and provides a SOCKS5 proxy interface for routing traffic through the anonymity network.

## Modules

| Module | Purpose |
|--------|---------|
| `client` | SOCKS5 client connections |
| `circuit` | Circuit building and management |
| `proxy` | Local proxy server |
| `control` | Control port communication |
| `installer` | Binary download and verification |
| `config` | Network configuration |

## Usage

```rust
use nonos_anyone::{AnyoneClient, ProxyConfig};

let client = AnyoneClient::new(ProxyConfig::default()).await?;
client.connect().await?;

// Traffic now routes through the network
let proxy_addr = client.socks_addr(); // 127.0.0.1:9050
```

## How It Works

1. Downloads Anyone Protocol binary (verified checksum)
2. Starts local SOCKS5 proxy
3. Builds encrypted multi-hop circuits
4. Routes all traffic through 3+ relays

No single relay sees both origin and destination.

## Configuration

```rust
ProxyConfig {
    socks_port: 9050,
    control_port: 9051,
    data_dir: ~/.nonos/anyone/,
}
```

## License

AGPL-3.0
