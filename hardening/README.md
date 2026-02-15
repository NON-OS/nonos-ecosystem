![NØNOS Banner](./assets/nonos-banner.png)

# NONOS Node Hardener

Server security hardening script for NONOS node operators.

## Quick Start

```bash
# Download and run
curl -fsSL https://raw.githubusercontent.com/NON-OS/nonos-ecosystem/main/hardening/nonos_hardening.sh | sudo bash
```

Or manually:

```bash
git clone https://github.com/NON-OS/nonos-ecosystem.git
cd nonos-ecosystem/hardening
chmod +x nonos_hardening.sh
sudo ./nonos_hardening.sh
```

## Supported Systems

- Ubuntu 22.04 LTS
- Ubuntu 24.04 LTS

## What It Does

| Step | Action |
|------|--------|
| 1 | System update and essential packages |
| 2 | SSH hardening (password auth enabled) |
| 3 | UFW firewall configuration |
| 4 | Fail2Ban brute-force protection |
| 5 | Kernel security hardening |
| 6 | Automatic security updates |
| 7 | Shared memory protection |
| 8 | Disable unused services |
| 9 | File permission hardening |
| 10 | Audit logging |
| 11 | Log rotation |
| 12 | NONOS service user creation |
| 13 | NONOS directory setup |
| 14 | Monitoring tools |
| 15 | Security status report |

## Ports

| Port | Protocol | Service |
|------|----------|---------|
| 22 | TCP | SSH (backup) |
| 54222 | TCP | SSH (primary) |
| 8420 | TCP | NONOS API |
| 9420 | TCP/UDP | NONOS P2P |

## SSH Access

After hardening, connect using:

```bash
# Primary (high port)
ssh root@your-server -p 54222

# Backup (if cloud provider blocks high ports)
ssh root@your-server -p 22
```

## Custom Ports

Override default ports with environment variables:

```bash
NONOS_API_PORT=9000 NONOS_P2P_PORT=9001 sudo ./nonos_hardening.sh
```

## Security Features

- **Fail2Ban**: Bans IPs after 5 failed SSH attempts (1 hour ban)
- **UFW Firewall**: Blocks all incoming except SSH and NONOS ports
- **Kernel Hardening**: Network security, ASLR, restricted dmesg
- **Auto Updates**: Automatic security patches
- **Audit Logging**: Monitors authentication and sudo usage

## After Hardening

Install and run the NONOS daemon:

```bash
# Install
cargo install nonos-daemon

# Initialize
nonos init

# Run
nonos run
```

## Logs

Hardening logs are saved to: `/var/log/nonos-harden.log`

## Troubleshooting

### Can't connect via SSH

1. Try port 22 (backup): `ssh root@server -p 22`
2. Use your hosting provider's web console/VNC
3. Check firewall: `ufw status`
4. Check SSH: `systemctl status ssh`

### Check what ports SSH is listening on

```bash
ss -tlnp | grep ssh
```

### Unban your IP from Fail2Ban

```bash
fail2ban-client set sshd unbanip YOUR_IP
```

## License

AGPL-3.0
