# Deployment Files

## Files

- `config.prod.toml` - Production configuration template

## VPS Setup

1. Create edgelord user:
   ```bash
   sudo useradd -r -s /bin/false edgelord
   sudo mkdir -p /opt/edgelord
   sudo chown edgelord:edgelord /opt/edgelord
   ```

2. Copy binary and config:
   ```bash
   sudo cp edgelord /opt/edgelord/
   sudo cp config.prod.toml /opt/edgelord/config.toml
   sudo chown edgelord:edgelord /opt/edgelord/*
   ```

3. Create .env file with secrets:
   ```bash
   sudo tee /opt/edgelord/.env << EOF
   WALLET_PRIVATE_KEY=0x...
   TELEGRAM_BOT_TOKEN=...
   TELEGRAM_CHAT_ID=...
   EOF
   sudo chmod 600 /opt/edgelord/.env
   sudo chown edgelord:edgelord /opt/edgelord/.env
   ```

4. Install and start service:
   ```bash
   sudo /opt/edgelord/edgelord service install
   sudo systemctl start edgelord
   ```

## Management

```bash
# View status
edgelord status

# View logs
edgelord logs -f

# Restart service
sudo systemctl restart edgelord

# Stop service
sudo systemctl stop edgelord
```
