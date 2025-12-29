#!/bin/bash
# ==============================================================
# NONOS Node Hardener v1.0
# Server Security Hardening for NONOS Node Operators
# Ubuntu 22.04 / 24.04 LTS
# ==============================================================
set -e

LOGFILE="/var/log/nonos-harden.log"
exec > >(tee -a "$LOGFILE") 2>&1

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# ==== NONOS LOGO ====
clear
echo -e "${GREEN}"
echo "███╗   ██╗ ██████╗ ███╗   ██╗ ██████╗ ███████╗"
echo "████╗  ██║██╔═══██╗████╗  ██║██╔═══██╗██╔════╝"
echo "██╔██╗ ██║██║   ██║██╔██╗ ██║██║   ██║███████╗"
echo "██║╚██╗██║██║   ██║██║╚██╗██║██║   ██║╚════██║"
echo "██║ ╚████║╚██████╔╝██║ ╚████║╚██████╔╝███████║"
echo "╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝"
echo -e "${NC}"
echo "        NONOS Node Hardener v1.0"
echo "============================================================="
echo ""

# Check root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}[!] This script must be run as root${NC}"
   exit 1
fi

# Detect OS
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    OS=$ID
    VERSION=$VERSION_ID
else
    echo -e "${RED}[!] Cannot detect OS${NC}"
    exit 1
fi

echo -e "${GREEN}[*]${NC} Detected: $OS $VERSION"
echo -e "${GREEN}[*]${NC} Hardening started at $(date)"
echo ""

# Port configuration
SSH_PORT=54222
NONOS_API_PORT=${NONOS_API_PORT:-8420}
NONOS_P2P_PORT=${NONOS_P2P_PORT:-9420}

echo -e "${GREEN}[*]${NC} SSH Port: $SSH_PORT (port 22 kept as backup)"
echo -e "${GREEN}[*]${NC} NONOS API port: $NONOS_API_PORT"
echo -e "${GREEN}[*]${NC} NONOS P2P port: $NONOS_P2P_PORT"
echo ""

########################################################
# 1. Update System and Install Essentials
########################################################
echo -e "${GREEN}[1/15]${NC} Updating system and installing essentials..."

export DEBIAN_FRONTEND=noninteractive
apt-get update && apt-get full-upgrade -y
apt-get install -y ufw fail2ban curl vim auditd unattended-upgrades \
    net-tools lsof gnupg2 bash-completion htop iotop

echo -e "${GREEN}[+]${NC} System updated."

########################################################
# 2. SSH Hardening (keeps port 22, password auth enabled)
########################################################
echo -e "${GREEN}[2/15]${NC} Hardening SSH..."

cp /etc/ssh/sshd_config /etc/ssh/sshd_config.bak.$(date +%Y%m%d)

# Remove any existing Port lines
sed -i '/^[[:space:]]*Port /d' /etc/ssh/sshd_config
sed -i '/^#Port /d' /etc/ssh/sshd_config

# Add ports right after Include line (this is the correct location)
sed -i '/^Include \/etc\/ssh\/sshd_config.d\/\*.conf/a Port 22\nPort '"$SSH_PORT" /etc/ssh/sshd_config

# Create drop-in for other hardening settings (not ports)
cat > /etc/ssh/sshd_config.d/99-nonos-hardening.conf <<EOF
# NONOS SSH Hardening
PermitRootLogin yes
PasswordAuthentication yes
PermitEmptyPasswords no
MaxAuthTries 5
MaxSessions 10
ClientAliveInterval 300
ClientAliveCountMax 2
X11Forwarding no
AllowAgentForwarding no
AllowTcpForwarding no
Banner /etc/issue.net
EOF

echo "Authorized access only. Disconnect immediately if unauthorized." > /etc/issue.net

# Test SSH config before restarting
if sshd -t; then
    # Ubuntu with systemd socket activation requires these commands
    systemctl daemon-reload
    systemctl restart ssh.socket 2>/dev/null || true
    systemctl restart ssh 2>/dev/null || systemctl restart sshd 2>/dev/null || true
    echo -e "${GREEN}[+]${NC} SSH hardened (password auth enabled)."
else
    echo -e "${RED}[!]${NC} SSH config error - reverting"
    rm -f /etc/ssh/sshd_config.d/99-nonos-hardening.conf
    exit 1
fi

########################################################
# 3. UFW Firewall Setup
########################################################
echo -e "${GREEN}[3/15]${NC} Configuring firewall..."

# CRITICAL: Allow SSH FIRST before doing anything else
ufw allow 22/tcp comment 'SSH Backup'
ufw allow $SSH_PORT/tcp comment 'SSH'

ufw --force reset
ufw default deny incoming
ufw default allow outgoing

# SSH (both ports - 22 as backup, 54222 as primary)
ufw allow 22/tcp comment 'SSH Backup'
ufw allow $SSH_PORT/tcp comment 'SSH'

# NONOS ports
ufw allow $NONOS_API_PORT/tcp comment 'NONOS API'
ufw allow $NONOS_P2P_PORT/tcp comment 'NONOS P2P'
ufw allow $NONOS_P2P_PORT/udp comment 'NONOS P2P UDP'

# Enable firewall
ufw --force enable

echo -e "${GREEN}[+]${NC} Firewall configured."

########################################################
# 4. Fail2Ban Configuration
########################################################
echo -e "${GREEN}[4/15]${NC} Configuring Fail2Ban..."

cat > /etc/fail2ban/jail.local <<EOF
[DEFAULT]
bantime = 1h
findtime = 10m
maxretry = 5
banaction = ufw

[sshd]
enabled = true
port = 22,$SSH_PORT
filter = sshd
logpath = /var/log/auth.log
maxretry = 5
bantime = 1h
EOF

systemctl enable fail2ban
systemctl restart fail2ban

echo -e "${GREEN}[+]${NC} Fail2Ban configured."

########################################################
# 5. Kernel Hardening
########################################################
echo -e "${GREEN}[5/15]${NC} Applying kernel hardening..."

cat > /etc/sysctl.d/99-nonos-hardening.conf <<EOF
# NONOS Kernel Hardening

# Network security
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.tcp_syncookies = 1
net.ipv4.conf.all.log_martians = 1
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.icmp_ignore_bogus_error_responses = 1

# System protections
fs.suid_dumpable = 0
kernel.randomize_va_space = 2
kernel.kptr_restrict = 2
kernel.dmesg_restrict = 1

# Performance for P2P
net.core.somaxconn = 4096
net.core.netdev_max_backlog = 4096
net.ipv4.tcp_max_syn_backlog = 4096
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_probes = 5
net.ipv4.tcp_keepalive_intvl = 15
EOF

sysctl -p /etc/sysctl.d/99-nonos-hardening.conf

echo -e "${GREEN}[+]${NC} Kernel hardened."

########################################################
# 6. Automatic Security Updates
########################################################
echo -e "${GREEN}[6/15]${NC} Enabling automatic security updates..."

cat > /etc/apt/apt.conf.d/50unattended-upgrades <<EOF
Unattended-Upgrade::Allowed-Origins {
    "\${distro_id}:\${distro_codename}";
    "\${distro_id}:\${distro_codename}-security";
    "\${distro_id}ESMApps:\${distro_codename}-apps-security";
    "\${distro_id}ESM:\${distro_codename}-infra-security";
};
Unattended-Upgrade::AutoFixInterruptedDpkg "true";
Unattended-Upgrade::Remove-Unused-Dependencies "true";
Unattended-Upgrade::Automatic-Reboot "false";
EOF

cat > /etc/apt/apt.conf.d/20auto-upgrades <<EOF
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
EOF

echo -e "${GREEN}[+]${NC} Automatic updates enabled."

########################################################
# 7. Secure Shared Memory
########################################################
echo -e "${GREEN}[7/15]${NC} Securing shared memory..."

if ! grep -q "/run/shm" /etc/fstab; then
    echo "tmpfs /run/shm tmpfs defaults,noexec,nosuid 0 0" >> /etc/fstab
fi

echo -e "${GREEN}[+]${NC} Shared memory secured."

########################################################
# 8. Disable Unused Services
########################################################
echo -e "${GREEN}[8/15]${NC} Disabling unused services..."

for service in cups avahi-daemon bluetooth; do
    if systemctl is-active --quiet $service 2>/dev/null; then
        systemctl stop $service
        systemctl disable $service
        echo "  Disabled: $service"
    fi
done

echo -e "${GREEN}[+]${NC} Unused services disabled."

########################################################
# 9. Set File Permissions
########################################################
echo -e "${GREEN}[9/15]${NC} Hardening file permissions..."

chmod 700 /root
chmod 600 /etc/ssh/sshd_config
chmod 644 /etc/passwd
chmod 644 /etc/group
chmod 600 /etc/shadow
chmod 600 /etc/gshadow

echo -e "${GREEN}[+]${NC} File permissions hardened."

########################################################
# 10. Configure Audit Logging
########################################################
echo -e "${GREEN}[10/15]${NC} Configuring audit logging..."

cat > /etc/audit/rules.d/nonos.rules <<EOF
# NONOS Audit Rules

# Monitor authentication
-w /etc/passwd -p wa -k identity
-w /etc/group -p wa -k identity
-w /etc/shadow -p wa -k identity
-w /var/log/auth.log -p wa -k auth_log

# Monitor sudo usage
-w /etc/sudoers -p wa -k sudoers
-w /etc/sudoers.d/ -p wa -k sudoers

# Monitor network config
-w /etc/hosts -p wa -k network
-w /etc/network/ -p wa -k network
EOF

systemctl enable auditd
systemctl restart auditd

echo -e "${GREEN}[+]${NC} Audit logging configured."

########################################################
# 11. Setup Log Rotation
########################################################
echo -e "${GREEN}[11/15]${NC} Configuring log rotation..."

cat > /etc/logrotate.d/nonos <<EOF
/var/log/nonos/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 root root
}
EOF

mkdir -p /var/log/nonos

echo -e "${GREEN}[+]${NC} Log rotation configured."

########################################################
# 12. Create NONOS User
########################################################
echo -e "${GREEN}[12/15]${NC} Creating nonos service user..."

if ! id -u nonos &>/dev/null; then
    useradd -r -s /usr/sbin/nologin -d /var/lib/nonos -m nonos
    echo -e "${GREEN}[+]${NC} User 'nonos' created."
else
    echo -e "${YELLOW}[*]${NC} User 'nonos' already exists."
fi

########################################################
# 13. Setup NONOS Data Directory
########################################################
echo -e "${GREEN}[13/15]${NC} Setting up NONOS directories..."

mkdir -p /var/lib/nonos
mkdir -p /etc/nonos
chown -R nonos:nonos /var/lib/nonos
chmod 700 /var/lib/nonos

echo -e "${GREEN}[+]${NC} NONOS directories created."

########################################################
# 14. Install Monitoring Tools
########################################################
echo -e "${GREEN}[14/15]${NC} Installing monitoring tools..."

apt-get install -y htop iotop nethogs iftop vnstat 2>/dev/null || true

echo -e "${GREEN}[+]${NC} Monitoring tools installed."

########################################################
# 15. Final Security Check
########################################################
echo -e "${GREEN}[15/15]${NC} Running final security check..."

echo ""
echo "============================================================="
echo -e "${GREEN}SECURITY STATUS${NC}"
echo "============================================================="
echo ""
echo -e "SSH Port:        ${GREEN}$SSH_PORT${NC} (22 as backup)"
echo -e "SSH Password:    ${GREEN}ENABLED${NC} (you won't be locked out)"
echo -e "Firewall:        ${GREEN}$(ufw status | head -1)${NC}"
echo -e "Fail2Ban:        ${GREEN}$(systemctl is-active fail2ban)${NC}"
echo -e "Auto Updates:    ${GREEN}enabled${NC}"
echo ""
echo "Open ports:"
ufw status numbered | grep -E "^\[" | head -10
echo ""

########################################################
# Complete
########################################################
echo ""
echo -e "${GREEN}"
echo "███╗   ██╗ ██████╗ ███╗   ██╗ ██████╗ ███████╗"
echo "████╗  ██║██╔═══██╗████╗  ██║██╔═══██╗██╔════╝"
echo "██╔██╗ ██║██║   ██║██╔██╗ ██║██║   ██║███████╗"
echo "██║╚██╗██║██║   ██║██║╚██╗██║██║   ██║╚════██║"
echo "██║ ╚████║╚██████╔╝██║ ╚████║╚██████╔╝███████║"
echo "╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝"
echo -e "${NC}"
echo ""
echo -e "${GREEN}[*] NONOS Node Hardening Complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
echo "  2. Load Rust: source \"\$HOME/.cargo/env\""
echo "  3. Clone repo: git clone https://github.com/NON-OS/nonos-ecosystem.git"
echo "  4. Build: cd nonos-ecosystem && cargo build --release -p nonos-daemon"
echo "  5. Install: sudo cp target/release/nonos-daemon /usr/local/bin/"
echo "  6. Run: nonos-daemon run"
echo ""
echo "Logs saved to: $LOGFILE"
echo ""
