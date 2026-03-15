mod discord_impl;
mod structs_json;
mod discord_commands;

use structs_json::DiscordInteraction;
use dotenvy::dotenv;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use reqwest::{Client, header::HeaderValue};
use std::{env, sync::Arc};

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};

#[derive(Clone)]
struct AppState {
    public_key: VerifyingKey,
    webhook_url: String,
    https: Client,
}

async fn handler_discord(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Paso 1. Extraer la firma y el timestamp de los headers
    println!("Recibiendo una solicitud de Discord...");

    let Some(signature) = headers.get("x-signature-ed25519") else {
        return (StatusCode::UNAUTHORIZED, "Falta la firma").into_response();
    };

    let Some(timestamp) = headers.get("x-signature-timestamp") else {
        return (StatusCode::UNAUTHORIZED, "Falta el timestamp").into_response();
    };

    // I want to merge timestamp_bytes and body into a single byte array, but I can't use the + operator directly on byte slices. Instead, I can create a new vector and extend it with both slices.
    // let combination_bytes = [timestamp_bytes, &body].concat();
    let combination_bytes = convert_package(timestamp, &body);

    // Paso 2. Verificar la firma usando la clave pública
    let Ok(signature_decoded) = hex::decode(&signature) else {
        return (StatusCode::UNAUTHORIZED, "Firma no válida").into_response();
    };

    // Convert it into a slice so then I can use verify function of ed25519_dalek
    let signature_slice = signature_decoded.as_slice();

    let Ok(signature_final) = Signature::from_slice(signature_slice) else {
        return (StatusCode::UNAUTHORIZED, "Firma no válida").into_response();
    };
    
    let Ok(_) = state
        .public_key
        .verify(&combination_bytes, &signature_final)
    else {
        println!("X Firma inválida");
        return (StatusCode::UNAUTHORIZED, "Firma no válida").into_response();
    };

    // Paso 3. Procesar el JSON
    let Ok(json_body) = serde_json::from_slice::<DiscordInteraction>(&body) else {
        return (StatusCode::BAD_REQUEST, "JSON no válido").into_response();
    };


    discord_impl::verify_command(&json_body, &state.webhook_url, &state.https)
        .await
        .into_response()
}

fn convert_package(timestamp: &HeaderValue, body: &Bytes) -> Vec<u8> {
    // Es referencia nomas, no mutable
    let timestamp_bytes = timestamp.as_bytes();
    let combination = [timestamp_bytes, body.as_ref()].concat(); // I create a new vector that concatenates the timestamp bytes and the body bytes. 

    combination
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(create_app_state()); // I create the application state and wrap it in an Arc for shared ownership across threads.
    println!("Estado inicializado y listo para usar");

    let app = create_app(shared_state); // I create the router and pass the shared state to it.

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Error, no se pudo bindear al puerto 3000");

    println!("🚀 Servidor seguro listo en el puerto 3000");

    axum::serve(listener, app)
        .await
        .expect("Error, el servidor no pudo iniciarse");
}

fn create_app_state() -> AppState {
    dotenv().ok(); // Carga las variables de entorno desde el archivo .env

    let public_key_str = env::var("DISCORD_PUBLIC_KEY").expect("DISCORD_PUBLIC_KEY no encontrado");

    let pub_key_bytes =
        hex::decode(&public_key_str).expect("Error, la clave publica no se pudo decodificar");

    let public_key = VerifyingKey::try_from(pub_key_bytes.as_slice())
        .expect("Error, la clave publica es invalida");

    let webhook_url = env::var("WEBHOOK_URL").expect("Falta URL del webhook");
    let https = Client::new();

    let state = AppState {
        public_key,
        webhook_url,
        https,
    };

    state
}

fn create_app(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/interactions", post(handler_discord))
        .with_state(shared_state) // I add the shared state to the router so that it can be accessed in the handler.
}
