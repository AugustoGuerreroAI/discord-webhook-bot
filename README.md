# Discord Webhook Bot (with rust)

A small, focused Discord interaction bot implemented from HTTP/interaction level (with no gateway)

**Purpose:** Learn and demonstrate async architecture, request verification, and clean command dispatching.

**Tech:** Built with Rust, Axum, Tokio and Reqwest. It interacts with the Discord HTTP Interactions API. During development I used Cloudflare to expose localhost and bc it's free lol (before that I used ngrok...)

---

### Features

- Verifying Discord interaction signatures (Ed25519)

- Immediate ACK + background processing (`tokio::main`) in order to avoid timeouts.

- Simple command dispatcher (`hola`, `sumar`, `insultar`).

- Shared `reqwest::Client` for connection reuse.

- **Clean separation:** verification **->** dispatcher **->** command modules :>

---

### Quickstart

1. Clone:
   
   ```rust
   git clone https://github.com/AugustoGuerreroAI/discord-webhook-bot.git
   cd discord-webhook-bot
   ```

2. Create .env (DO NOT COMMIT IT) and set at least:
   
   ```markdown
   DISCORD_PUBLIC_KEY=<your_discord_public_key>
   WEBHOOK_URL=<https://discord.com/api/webhooks/{app_id}/{interaction_token}>
   
   Bot identity (optional, but useful just in case)
   DISCORD_TOKEN="your discord token"
   ```

3. Build & run:
   
   ```
   cargo build --release
   cargo run --release
   ```

> During dev you can expose `http://localhost:3000` with **Cloudflare Tunner** (or ngrok) and set that URL as the Discord interaction endpoint!!!

---

## Files of interest

- `src/main.rs` -- app bootstrap & `Appstate` creation

- `src/discord_impl.rs` -- Interaction verification and dispatcher

- `src/discord_commands.rs` -- Command implementations

- `src/structs_json.rs` -- serde models for Discord payloads

- `docs/devlog.md` -- devlog / daily notes (RECOMMENDED READING PLZ)

---

### <u>Notes + best practices </u>🗿🗿

- DO NOT COMMIT `.env` (secrets). `.gitignore` already excludes it.

- Use the singe `reqwest::Client` stored in `Appsate`(cloning the client is cheap bc it shares the pool).

- `tokio::spawn` requires owned data (`static) for background things (tasks); clone handles into the closure.

- Consider a command registry (e.g., `HashMap<String, Handler>`) if commands grow...

---

## Next steps / ideas xd

- Replace the goddam `match` dispatcher with a command registry trait for pluggable commands...

- Add tests for signature verification and command parsing.

- Dockerize and add a basic CI pipeline.

---
### Goals
- Implement an economy system with sqlite in which registers importan activity from the user that interact with the bot.


#### License and contribution

Open for contributions 🗿🗿🗿
