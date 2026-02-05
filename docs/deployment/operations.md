# Operations

Monitoring, security hardening, and troubleshooting for production.

## Monitoring

### Log Management

Configure log rotation:

```bash
sudo nano /etc/logrotate.d/edgelord
```

```
/var/log/edgelord/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
}
```

### Health Checks

Simple health check script:

```bash
#!/bin/bash
# /home/edgelord/health-check.sh

if ! systemctl is-active --quiet edgelord; then
    echo "Edgelord is down!" | telegram-send --stdin
    sudo systemctl restart edgelord
fi
```

Add to crontab:

```bash
crontab -e
```

```
*/5 * * * * /home/edgelord/health-check.sh
```

### Status Commands

```bash
# Service status
systemctl status edgelord

# Live logs
journalctl -u edgelord -f

# Bot status
./target/release/edgelord status

# Recent logs
journalctl -u edgelord --since "1 hour ago"
```

## Security Checklist

- [ ] Non-root user for running the bot
- [ ] Private key in `.env` file with `chmod 600`
- [ ] Config file with `chmod 600`
- [ ] Firewall enabled, only SSH open
- [ ] SSH key authentication only
- [ ] Fail2ban installed
- [ ] Dedicated trading wallet (not main holdings)
- [ ] Start with small position limits

### Firewall Setup

```bash
# Enable UFW
sudo ufw allow OpenSSH
sudo ufw enable

# Verify
sudo ufw status
```

### SSH Hardening

```bash
sudo nano /etc/ssh/sshd_config
```

Set:
```
PasswordAuthentication no
PermitRootLogin no
```

Restart:
```bash
sudo systemctl restart sshd
```

### Fail2ban

```bash
sudo apt install fail2ban
sudo systemctl enable fail2ban
sudo systemctl start fail2ban
```

## Troubleshooting

### Connection Issues

```
Error: WebSocket connection failed
```

- Check Polymarket reachable: `curl -I https://clob.polymarket.com`
- Verify non-US IP: `curl ifconfig.me`
- Check outbound firewall: `sudo ufw status`
- Check DNS resolution: `nslookup clob.polymarket.com`

### Wallet Issues

```
Error: Insufficient balance
```

- Check USDC balance on Polygon
- Check MATIC balance for gas
- Verify correct network (Polygon mainnet, chain ID 137)
- Check on [Polygonscan](https://polygonscan.com/)

```
Error: Transaction failed
```

- Check gas price isn't spiking
- Verify USDC approval: `./target/release/edgelord wallet approve --amount 1000`
- Check wallet isn't flagged/blocked

### Performance Issues

```
Warning: Detection latency > 100ms
```

- Check server load: `htop`
- Check network latency: `ping clob.polymarket.com`
- Consider upgrading VPS specs
- Check for memory pressure: `free -m`

### Service Issues

```bash
# Service won't start
journalctl -u edgelord -n 50

# Check config syntax
./target/release/edgelord check config

# Permission issues
ls -la ~/.config/edgelord/

# Environment not loading
systemctl show edgelord | grep EnvironmentFile
```

## Quick Reference

| Command | Description |
|---------|-------------|
| `systemctl status edgelord` | Service status |
| `systemctl restart edgelord` | Restart bot |
| `systemctl stop edgelord` | Stop bot |
| `journalctl -u edgelord -f` | Follow logs |
| `./target/release/edgelord status` | Bot status |
| `curl ifconfig.me` | Check IP location |
| `htop` | Server load |

## Updating

```bash
cd ~/edgelord
git pull
cargo build --release
sudo systemctl restart edgelord
```

## Backup

Important files to backup:

```bash
~/.config/edgelord/config.toml
~/.config/edgelord/.env
```

**Never** commit `.env` to git. Keep private key backed up securely elsewhere.
