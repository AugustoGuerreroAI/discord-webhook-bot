use crate::structs_json::{DiscordInteractionData, DiscordMember};
use reqwest::Client;
use serde_json::{ Value, json };
use reqwest::Response;
use reqwest::Error;

pub async fn hola(data: &Option<DiscordMember>, webhook_url: &str, https: &Client) {

    if let Some(member) = data {
        let message: String = format!("Hola {}", member.user.username);
        let payload_webhook = create_payload_webhook(message);
        let response = send_response(payload_webhook, webhook_url, https).await;

        check_response(response).await;
    } else {
        println!("No se pudo realizar el comando 'hola' ");
    }

}

pub async fn sumar(data: &DiscordInteractionData, webhook_url: &str, https: &Client) {

    // data.options are the arguments of the function, inside 
    // options there is a Vec of DiscordCommandOption, DiscordCommandOption has a name and a value, value is the var that we're going to use

    let mut total: u32 = 0;
    let mut broken: bool = false;

    if let Some(opts) = &data.options {
        for opt in opts {
            if let Some(value) = &opt.value { // that means that value is available
                if let Some(value_converted) = value.as_u64() {
                    total = total + value_converted as u32;
                } else {
                    broken = true;
                    break;
                }
            } else {
                broken = true;
                break;
            }
        }
    }

    let response = if !broken {
            format!("La suma entre todos los numeros recibidos es: {}", total)
        } else {
            format!("La suma no se pudo realizar, uno o mas argumentos son invalidos...")
        };

    let payload_webhook = create_payload_webhook(response);

    let res = send_response(payload_webhook, webhook_url, https).await;
    check_response(res).await;
}

pub async fn insultar(data: &DiscordInteractionData, webhook_url: &str, https: &Client) {
    // Primer argumento = ping
    // Segundo argumento = mensaje

    if let Some(opts) = &data.options {

        if let (Some(raw_mention), Some(raw_message)) = (&opts[0].value, &opts[1].value) {
            if let (Some(mention), Some(message)) = (raw_mention.as_str(), raw_message.as_str()) {
                // Now we have here the real value of mention and message. We can create and send the message :)
                let response = format!("# <@{}>, {}", mention, message);
                let payload_webhook = create_payload_webhook(response);
                let res = send_response(payload_webhook, webhook_url, https).await;
                check_response(res).await;
            } else {
                println!("Alguno de los dos argumentos no se pudo pasar a string...");
            }
        } else {
            println!("Insuficientes argumentos");
        }
    }
}

// Misc functions

fn create_payload_webhook(nombre: String) -> Value {
    json!({
        "content": nombre,
    })
}

async fn send_response(payload_webhook: Value, webhook_url: &str, https: &Client) -> Result<reqwest::Response, reqwest::Error> {
    println!("ENVIANDO RESPUESTA...");

    https.post(webhook_url)
        .json(&payload_webhook)
        .send()
        .await
}

pub async fn check_response(res: Result<Response, Error>) {
    if let Ok(_) = res {
        println!("El comando se ha enviado con exito ✅✅✅");
    } else {
        println!("El comando no se pudo realizar... ❌❌❌");
    }
}