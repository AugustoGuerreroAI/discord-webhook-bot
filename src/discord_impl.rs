use axum::Json;
use reqwest::Client;
use serde_json::{ Value, json };
use crate::discord_commands;

use crate::structs_json:: { DiscordInteraction };


pub async fn verify_command(json_body: &DiscordInteraction, webhook_url: &str, https: &Client) -> Json<Value> {
    // Aca podes implementar la logica para verificar el comando y responder usando el webhook
    println!("Verificando comando recibido de Discord...");

    match json_body.interaction_type {
        1 => { // Cuando es un ping
            return Json(json!({"type": 1}));
        }

        2 => {
            println!("esto es un comando, ojo");
            let body_clone = json_body.clone();
            let webhook_clone = webhook_url.to_string();
            let client_clone = https.clone();

            tokio::spawn(async move {
                check_interaction_data(&body_clone, &webhook_clone, client_clone).await;
            });

            return Json(json!({
                "type": 4,
                "data": { "content": "🔃 Procesando comando..." }
            }));
        }

        _ => {
            return Json(json!({
                "type": 4,
                "data": {
                    "content": "Ocurrio algo inesperado...",
                }
            }));
        }
    }
}


async fn check_interaction_data(body: &DiscordInteraction, webhook_url: &str, https: Client) {

    if let Some(data) = &body.data { // This means that if body has "data" header, then do this:
        println!("name of the command: {}", data.name);

        match data.name.as_str() {
            "hola" => discord_commands::hola(&body.member, webhook_url, &https).await,
            "sumar" => discord_commands::sumar(&data, webhook_url, &https).await,
            "insultar" => discord_commands::insultar(&data, webhook_url, &https).await,
            _ => println!("No existe ese comando xdxddx"),
        }
    }

}