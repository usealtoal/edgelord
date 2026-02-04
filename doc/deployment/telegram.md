# Telegram Alerts

Set up a Telegram bot to receive alerts for opportunities, trades, and errors.

## Step 1: Create a Bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot`
3. Choose a display name: `Edgelord Alerts`
4. Choose a username: `your_edgelord_bot` (must end in `bot`, be unique)
5. BotFather replies with your token:
   ```
   Use this token to access the HTTP API:
   7123456789:AAHxxxxxxxxxxxxxxxxxxxxxxxxxxx
   ```

Save this — it's your `TELEGRAM_BOT_TOKEN`.

## Step 2: Set Profile Picture

1. In BotFather chat, send `/setuserpic`
2. Select your bot from the list
3. Send a square image (512x512px recommended)
4. BotFather confirms the update

## Step 3: Customize Bot (Optional)

Still in BotFather:

```
/setdescription   — Short description shown when opening the bot
/setabouttext     — Longer about text
/setcommands      — Command menu (not needed for alert-only bot)
```

## Step 4: Get Your Chat ID

The bot needs to know where to send messages.

### Personal Chat

1. Start a chat with your bot (search for it, click Start)
2. Send any message (e.g., "hello")
3. Open in browser (replace TOKEN):
   ```
   https://api.telegram.org/bot<TOKEN>/getUpdates
   ```
4. Find your chat ID in the response:
   ```json
   "chat": {
     "id": 123456789,
     "first_name": "Your Name",
     "type": "private"
   }
   ```

### Group Chat

1. Add the bot to your group
2. Send a message in the group
3. Check `getUpdates` — group IDs are negative (e.g., `-987654321`)

## Step 5: Configure Environment

Add to `~/.config/edgelord/.env`:

```bash
TELEGRAM_BOT_TOKEN=7123456789:AAHxxxxxxxxxxxxxxxxxxxxxxxxxxx
TELEGRAM_CHAT_ID=123456789
```

## Step 6: Configure Alerts

In `config.toml`:

```toml
[notifications.telegram]
enabled = true
min_profit_alert = 1.00       # Only alert for profits > $1
alert_on_error = true         # Alert on errors/restarts
alert_on_opportunity = true   # Alert when opportunity detected
alert_on_execution = true     # Alert when trade executes
```

## Step 7: Test

```bash
./target/release/edgelord test-telegram
```

You should receive a test message.

## Example Messages

**Opportunity detected:**
```
🔍 Opportunity Found
Market: Will Bitcoin hit $100k?
Strategy: single_condition
Edge: 3.2% ($4.80)
Volume: 150 shares
```

**Trade executed:**
```
✅ Trade Executed
Market: Will Bitcoin hit $100k?
Profit: $4.52
Legs: 2/2 filled
```

**Error:**
```
⚠️ Error
WebSocket disconnected, reconnecting...
```

## Troubleshooting

### Bot not responding

- Verify token is correct
- Make sure you started a chat with the bot first
- Check `getUpdates` returns your message

### Wrong chat ID

- Chat IDs are numbers, not usernames
- Personal chats: positive number
- Group chats: negative number
- Re-check `getUpdates` response

### No alerts received

- Verify `enabled = true` in config
- Check edgelord logs for notification errors
- Test with `./target/release/edgelord test-telegram`
