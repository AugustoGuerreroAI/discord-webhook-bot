# Devlog

## 3/4/26 — Understanding how to host the bot

Ok so to host the bot correctly and receive slash command requests from users I need to use the Tokio infra.  

Basically I open port 3000 using `TcpListener` with Tokio so the server can handle concurrency. The bot should not wait until finishing one command before accepting another request.  

So the idea is that multiple requests can arrive at the same time and the runtime handles them async.  

---  

## 3/4/26 — Understanding TcpListener + Axum

Ok now I think I get it.  

`tokio::net::TcpListener("0.0.0.0:3000")` is basically the address where the server listens.  

Axum is the one that processes the HTTP requests using `axum::serve`, but it still needs a listener first.  

**So something like this:  **

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
```

The `await` is there because binding the socket is async and we need to wait until the OS actually opens the port.

The `unwrap` just unwraps the result. But yeah if something fails Rust will panic.

At first I thought `expect` was just used to print a custom error message, but it also panics the program. So it’s better practice because at least you know what the hell failed instead of searching line by line where the program died.

## 3/5/26 — Continuing the Discord handler

Working on the `handler_discord` function.  

```rust
async fn handler_discord( 
    State(state): State<Arc<AppState>>, 
    headers: HeaderMap, 
    body: Bytes, 
) -> impl IntoResponse
```

Here I destructure the `State` wrapper provided by Axum.

This gives access to `Arc<AppState>`. Since it is wrapped in `Arc`, ownership can safely be shared across requests. Multiple async tasks can access the same state without violating Rust's ownership rules.

---

## 3/5/26 — Discord request security

Discord requires verifying that requests actually come from their servers.

This means verifying a signature using the public key they provide.  
The verification process requires combining:

- the timestamp

- the raw request body

Then verifying the signature using `ed25519`.

The steps are basically:

1. Extract the signature and timestamp from the headers

2. Combine timestamp + body

3. Decode the hex signature

4. Verify the signature using Discord's public key

Example of the header extraction:

```rust
let Some(signature) = headers.get("x-signature-ed25519") else {
    return (StatusCode::UNAUTHORIZED, "Missing signature").into_response();
};

let Some(timestamp) = headers.get("x-signature-timestamp") else {
    return (StatusCode::UNAUTHORIZED, "Missing timestamp").into_response();
};
```

---

## 3/5/26 — Combining timestamp and body

I needed to merge the timestamp bytes with the request body.

You can't simply use the `+` operator on byte slices, so instead I created a new vector containing both.

```rust
let combination_bytes = convert_package(timestamp, &body);
```

This creates the exact byte sequence Discord expects when verifying the signature.

---

## 3/5/26 — Decoding the signature

The signature arrives encoded as hex.

So the next step is decoding it:

```rust
let Ok(signature_decoded) = hex::decode(&signature) else {
    return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
};
```

After decoding it, I convert it into a slice so it can be used by `ed25519_dalek`.

At first I tried using `Signature::from_bytes`, but that expects a fixed `[u8; 64]` array.  
What I had instead was `&[u8]`.

Switching to `Signature::from_slice` solved that issue.

```rust
let signature_slice = signature_decoded.as_slice();

let Ok(signature_final) = Signature::from_slice(signature_slice) else {
    return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
};
```

---

## 3/5/26 — Signature verification

Finally I can verify that the request was actually signed by Discord.

```rust
let Ok(result) = state.public_key.verify(&combination_bytes, &signature_final) else {
    println!("Invalid signature");
    return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
};
```

If the verification succeeds, the request can be trusted.

```rust
(StatusCode::OK, "Signature verified successfully").into_response()
```

At this point the request passed all required checks and it is safe to process the command payload.

---

## 3/5/26 — Next step: parsing the JSON payload

Now that the request is verified, the next step is parsing the JSON body sent by Discord.

For this I will use `serde`, which is the standard crate in Rust for serialization and deserialization.

Quick reminder:

**Serialization**  
Rust data structure → external format (JSON, TOML, etc.)

**Deserialization**  
External format → Rust data structure

In this case I need to deserialize the JSON interaction payload sent by Discord.

---

## 3/5/26 — End of session

Stopping here for now, need to leave soon for class.

Next step will be implementing the JSON parsing and command handling logic.

---

## 3/6/26 — Parsing the Discord interaction JSON

To process the request body sent by Discord I need to deserialize the JSON payload.

The first step is creating a Rust struct that represents the data I want to extract.

### New concept: `type` alias

While experimenting I ran into another Rust concept: `type`.

A `type` alias allows creating a new name for an existing type.  
It doesn't create a new type, it just improves readability.

Example idea:

```rust
type UserId = u64;
```

This helps communication in the code by giving meaning to the value.

---

## Problem: JSON field called `type`

Discord's interaction payload includes a field called `"type"`.

However `type` is a reserved keyword in Rust, so it cannot be used directly as a struct field name.

To solve this I used Serde's rename attribute.

```rust
#[derive(Deserialize)]  
struct DiscordPing {  
    #[serde(rename = "type")]  
    interaction_type: String,  
}
```

The `#[derive(Deserialize)]` attribute gives the struct the ability to be constructed from JSON.

The `#[serde(rename = "type")]` attribute tells Serde:

> "Take the value from the JSON field `type` and store it in `interaction_type`."

---

### Adjusting the data type

After checking the Discord documentation I noticed that the interaction type is actually a numeric value.

So I changed the field type to `u8`:

```rust
#[derive(Deserialize)]  
struct DiscordPing {  
    #[serde(rename = "type")]  
    interaction_type: u8,  
}
```

This is more accurate and avoids unnecessary conversions.

---

## 3/6/26 — Deserializing the JSON body

To deserialize the request body I used `serde_json`.

Initial attempt:

```rust
let Ok(json_body): DiscordPing = serde_json::from_slice(&body) else {
    return (StatusCode::BAD_REQUEST, "JSON no válido").into_response();
};
```

This did not compile because the compiler couldn't infer the generic type.

The solution was using Rust's **turbofish syntax**.

```rust
let Ok(json_body) = serde_json::from_slice::<DiscordPing>(&body) else {
    return (StatusCode::BAD_REQUEST, "JSON no válido").into_response();
};
```

### Turbofish syntax

This syntax looks like:

```rust
    function::(arguments)
```

It explicitly tells the compiler which concrete type a generic function should return.

In this case it tells `serde_json::from_slice` to deserialize the JSON into `DiscordPing`.

---

## 3/6/26 — Exposing the local server to Discord

To allow Discord to send requests to my local server I needed a public endpoint.

Originally I used **ngrok**, but I decided to switch to **Cloudflare Tunnel**, which allows creating temporary public URLs without paying.

Command used:

```rust
cloudflared tunnel --url http://localhost:3000 
```

This generates a temporary public URL that forwards traffic to the local server.

Example output:

```
https://something.trycloudflare.com (https://something.trycloudflare.com/)
```

That URL is then configured inside the Discord Developer Portal as the interaction endpoint.

---

## 3/6/26 — Discord endpoint verification

When configuring the interaction endpoint, Discord performs two verification requests:

1. A valid request
2. An invalid request

The server must respond correctly to both in order to pass the verification process.

After implementing the signature verification and JSON parsing earlier, the server passed this check successfully.

At this point the interaction endpoint is working and Discord can send commands to the bot.

---

## 3/6/26 — Inviting the bot to the server

To invite the bot:

1. Go to **OAuth2 → URL Generator**
2. Select the scope `applications.commands`
3. Open the generated URL
4. Invite the bot to the server

Once invited, the bot was able to receive commands and respond successfully.

Example response during testing:

```markdown
Command received and verified successfully.
```

This confirms that the request pipeline works end-to-end.

---

## 3/8/26 — Testing after distro migration

Finished migrating my development environment to the new Linux distro.  
Next step was recreating the Cloudflare tunnel so Discord could reach my local server again.

Started the tunnel:

```
cloudflared tunnel --url [http://localhost:3000](http://localhost:3000/)
```

This generated a public endpoint like:

```
https://something.trycloudflare.com/(https://something.trycloudflare.com/)
```

The correct endpoint for Discord interactions must include the route:

```
https://something.trycloudflare.com/interactions
```

After configuring it in the Discord Developer Portal the server started receiving requests again.

Server output looked like this:

```
Secure server ready on port 3000  
Receiving request from Discord...  
Invalid signature  
Receiving request from Discord...  
Verifying command from Discord...  
This is a ping, responding with type 1
```

Discord first sends verification requests to confirm the endpoint is valid.  
Once that succeeded the bot responded correctly.

Test response:

```
Command received and verified successfully
```

---

## 3/8/26 — Moving from Ping to Interaction

Initially the struct I created only handled the `Ping` interaction.

Now I need to support full **Discord interactions**.

So I renamed the struct from `DiscordPing` to `DiscordInteraction`.

The key detail is that Discord sends different interaction types:

- `type = 1` → Ping
- `type = 2` → Application command

Type 2 interactions contain much more data, including information about the user that executed the command.

Example structure:

```rust
#[derive(Deserialize)]
pub struct DiscordInteraction {
    #[serde(rename = "type")]
    pub interaction_type: i8,
    member: Option<Member>
}
```

The `member` field is wrapped in `Option` because it only exists for certain interaction types.

---

## Inspecting the raw JSON

To understand what Discord actually sends I printed the raw JSON body when executing a simple `/hola` command.

The payload is extremely large and contains a lot of information:

- application id

- channel info

- guild info

- permissions

- user data

- roles

- tokens

Even a simple command sends a large structure.

However, for the current bot logic I only need a very small portion of it.

The most relevant part is:

```
member → user
```

Example:

```json
member: {
  user: {
    id: "...",
    username: "...",
    global_name: "..."
  }
}
```

So the bot can identify which user triggered the command.
---

## Command verification logic

Once the interaction is verified the bot processes the request.

Current flow:

```rust
pub async fn verify_command(json_body: &DiscordInteraction, webhook_url: &str) -> Json<Value> {
```

1. If interaction type is **1** → respond to Discord Ping.

2. If interaction type is **2** → process a command.

Example logic:

```rust
if json_body.interaction_type == 1 {
    return Json(json!({ "type": 1 }));
}
```

For commands:

```rust
if let Some(member_data) = &json_body.member {
```

Using `if let Some(...)` safely unwraps the optional member data.

Inside that block is where the bot logic can be implemented.

---

## First dynamic response

To test everything I made the bot greet the user who triggered the command.

```rust
let nombre_personalizado: String =
    format!("Hola {}", &member_data.user.username);
```

Response sent back to Discord:

```rust
Json(json!({
    "type": 4,
    "data": {
        "content": nombre_personalizado
    }
}))
```

---

## Result

After restarting the server and testing the command:

Bot response:

```
Hola lozi_25
```

Which confirms:

- JSON parsing works

- interaction verification works

- user data extraction works

- response pipeline works

---

# Devlog: Refactoring, Webhooks, and Slash Commands

**Date: March 9, 2026**

## [6:01 PM] Structuring the JSON Data

To properly handle incoming interactions from Discord, I needed to map the JSON payload to Rust structs. The main fields required are `interaction_type`, `data`, and `member`.

Because some fields like `data` or `options` might not always be present, I wrapped them in Rust's `Option` enum and used `Vec` for lists. Here is the updated structure using `serde`:

```Rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DiscordInteraction {
    #[serde(rename = "type")]
    pub interaction_type: i8,
    pub data: Option<DiscordInteractionData>,
    pub member: Option<DiscordMember>,
}

#[derive(Deserialize)]
pub struct DiscordInteractionData {
    pub name: String,
    pub options: Option<Vec<DiscordCommandOption>>,
}

#[derive(Deserialize)]
pub struct DiscordMember {
    pub user: UserData,
}

#[derive(Deserialize)]
pub struct UserData {
    pub username: String,
}

#[derive(Deserialize)]
pub struct DiscordCommandOption {
    pub name: String,
    pub value: String,
}
```

## [6:53 PM - 7:02 PM] Code Refactoring & Webhook Implementation

I spent some time refactoring the codebase to make it cleaner. This process really helped solidify my understanding of core Rust concepts like the turbofish syntax, `await`, `Option`, and `Result`.

I updated the `verify_command` function to match the `interaction_type`. If the type is `1`, it responds to Discord's mandatory ping. If it's `2`, it processes the command.

I also implemented the webhook response logic (`check_member_data` and `send_response`). The bot now successfully extracts the user's name from the payload and sends a personalized greeting back through the webhook. *(Note: fixed a bug where a missing `.await` was preventing the webhook from firing).*

## [7:17 PM - 7:45 PM] Registering New Slash Commands

I documented the process for registering new commands with Discord's API. To create a new slash command, a `POST` request must be sent to:

`https://discord.com/api/v10/applications/{APPLICATION_ID}/commands`

The request requires the bot token in the headers (`Authorization: Bot <TOKEN>`) and a JSON body defining the command. Here is the payload tested for a `/sumar` command with arguments:

```json
{
  "name": "sumar",
  "description": "Suma dos numeros",
  "options": [
    {
      "name": "a",
      "description": "primer numero",
      "type": 4,
      "required": true
    },
    {
      "name": "b",
      "description": "segundo numero",
      "type": 4,
      "required": true
    }
  ]
}
```

## [7:55 PM] Next Steps: Modularizing Command Logic

Testing was successful, and the application can now explicitly read the executed command's name (e.g., "hola") directly from the JSON payload.

To prevent the `match` statement from getting too large and difficult to maintain, the next logical step is to modularize the application. I will move the implementation details (`check_interaction_data`) into a separate file. This will allow for scaling the bot, easily adding new commands and their respective functions in a structured way.

---

Here is the professional, clean, and translated version of your latest session, with the timestamps included and all the obscenities filtered out. I maintained the technical focus on the exact concepts you tackled (like `serde_json::Value` and `tokio::spawn`) to ensure the devlog remains highly useful for your project.

---

# Devlog: Bug Fixing, Type Parsing, and Concurrency

**Date: March 11, 2026**

## [6:20 PM] Statement vs. Expression Bug Fix

I caught a minor syntax issue that was preventing the response string from evaluating correctly. By accidentally leaving a trailing semicolon `;` at the end of an `if/else` block, I had converted an expression into a statement, meaning it wasn't returning the expected value to the `response` variable.

I corrected the logic to properly format the response before sending it to the webhook:

```rust
let response = if !broken {
    format!("La suma entre todos los numeros recibidos es: {}", total)
} else {
    format!("La suma no se pudo realizar, uno o mas argumentos son invalidos...")
};

let payload_webhook = create_payload_webhook(response).await;
let res = send_response(payload_webhook, webhook_url).await;
check_response(res).await;
```

## [6:43 PM - 7:34 PM] Debugging JSON Deserialization Errors

While testing the new command, the bot kept hanging at the "Processing command..." stage. After an hour of debugging, I found the root cause: strict typing and JSON deserialization.

Because I defined the slash command option as an integer (`type: 4`), Discord was sending a raw number. However, my Rust struct was expecting an `Option<String>`. The `serde` crate is strictly typed, so it couldn't automatically fit an integer into a `String` field, causing the entire JSON processing pipeline to fail silently.

## [7:39 PM] Fixing Type Mismatches with `serde_json::Value`

To fix this, I updated the struct to use `Option<serde_json::Value>` instead of `String`. `Value` acts as a universal container, allowing the struct to safely catch any data type (strings, integers, etc.) that Discord might send. Once captured, I can use `serde`'s built-in helper functions to securely parse the exact type I need.

**Before (Failing on numbers):**

```rust
if let Some(value) = &opt.value { 
    if let Ok(value_converted) = value.parse::<u32>() {
        total = total + value_converted;
// ...
```

**After (Successfully using `.as_u64()`):**

```rust
if let Some(value) = &opt.value {
    if let Some(num) = value.as_u64() {
        total += num as u32;
    } else {
        broken = true;
        break;
    }
} else {
    broken = true;
    break;
}
```

## [7:39 PM - 7:47 PM] Maximizing Concurrency with Tokio

To ensure the bot never times out and triggers Discord's "didn't respond in time" error, I refactored the interaction handler (`match 2`).

I implemented `tokio::spawn` to offload the heavy logic (`check_interaction_data`) to a separate background thread. By cloning the JSON body and webhook URL, the main thread remains completely unblocked. It immediately returns a fast "🔃 Procesando comando..." JSON response to Discord, while the background thread handles the actual math and webhook execution. This is a highly efficient, scalable approach for handling bot interactions.

```rust
2 => {
    println!("esto es un comando, ojo");
    let body_clone = json_body.clone();
    let webhook_clone = webhook_url.to_string();

    tokio::spawn(async move {
        check_interaction_data(&body_clone, &webhook_clone).await;
    });

    return Json(json!({
        "type": 4,
        "data": { "content": "🔃 Procesando comando..." }
    }));
}
```

---
# Devlog: User Mentions, Snowflake IDs, and Pattern Matching

**Date: March 13, 2026**

## [6:14 PM - 6:22 PM] Designing the Custom Mention Command

Today's goal was to implement a command that takes two arguments: a target user to mention and a custom message.

When constructing the payload to send back to Discord, I learned that you cannot simply send the username. Discord requires a specific formatting syntax to trigger a ping: `<@USER_ID>`. Additionally, when registering the command options with Discord's API, I had to ensure the argument types were correct (Type 3 for Strings, Type 6 for User Mentions) and that the argument names contained no uppercase letters.

## [6:30 PM] Handling Discord Snowflake IDs

I encountered a technical catch with how Discord handles User IDs.

Discord IDs are called "Snowflakes." Because these numbers are massive (my ID, for example, is `1371245087128027158`, which far exceeds the 4-billion limit of a `u32` integer), languages like JavaScript lose precision if they try to read them as standard numbers. To prevent this, Discord always sends these IDs formatted as strings within the JSON payload (e.g., `"1371245087128027158"`).

Using `serde_json`'s `.as_u64()` method returns `None` because the parser detects a string, not a raw integer.

## [6:40 PM] Stripping Quotes from `serde_json::Value`

Because the extracted `value_user` variable is a `serde_json::Value` container, passing it directly into a `format!` macro causes Rust to print it exactly as it appears in the JSON—quotes included.

* **Direct formatting:** `format!("<@{}>", value_user)` results in `<@"123456789">` (Invalid mention).
* **Using `.as_str()`:** Unwraps the JSON container and extracts the raw string characters without the quotes, resulting in the correct `<@123456789>` format.

## [6:47 PM] Refactoring: Replacing Nested `if let` with Tuples

Extracting the `Option` values from the command arguments initially led to deeply nested `if let` blocks, which made the code difficult to read. I refactored this using Rust's tuple pattern matching to evaluate multiple `Option` variables simultaneously.

This significantly cleaned up the logic:

**Before (Deep nesting):**

```rust
if let Some(value_message) = &opts[0].value { 
    if let Some(value_user) = &opts[1].value { 
        if let Some(value_converted) = value_user.as_str() { 
            if let Some(message_converted) = value_message.as_str() { 
                // Logic...
            }
        }
    }
}

```

**After (Clean Tuple Matching):**

```rust
if let Some(opts) = &data.options {
    if let (Some(raw_mention), Some(raw_message)) = (&opts[0].value, &opts[1].value) {
        if let (Some(mention), Some(message)) = (raw_mention.as_str(), raw_message.as_str()) {
            
            // Successfully extracted raw strings, constructing the payload
            let response = format!("<@{}>, {}", mention, message);
            let payload_webhook = create_payload_webhook(response).await;
            
            let res = send_response(payload_webhook, webhook_url).await;
            check_response(res).await;
        }
    }
}

```

## [6:54 PM] Final Testing

The refactored code successfully executed. The only hiccup during final testing was realizing I had forgotten to save and update the command schema on Discord's side to reflect the new string types instead of integers. Once updated, the bot successfully mentioned the target user with the custom message.

# Devlog: Async command heandling and HTTP Client Reuse [15/3/26]
Focused on imporving internal architecture of the Discor interaction handler and solving lifetime issue related to async task execution

Interactions are now processed through a clear pipeline: the server receives the request, validates it, and passes it to `verify_command`. Ping interactions respond immediately, while application commands spawn a background task so the server can reply quickly with a "processing" message.

While implementing this, I ran into a lifetime issue when passing `&Client` into `tokio::spawn`. Since spawned tasks must be `'static`, references tied to the current function cannot be captured. The fix was to clone the HTTP client before spawning the task. Since the client internally shares its connection pool, cloning it is inexpensive.

Commands are dispatched through a simple `match` based on the command name (`hola`, `sumar`, `insultar`), with each command implemented in a separate module. I also added small helper utilities for building webhook payloads and sending responses.

The system now supports asynchronous command execution, reuses a shared HTTP client, and keeps interaction handling separate from command logic!!!.


