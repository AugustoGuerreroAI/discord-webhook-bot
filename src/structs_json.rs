use serde::{ Deserialize };
use serde_json::Value;


#[derive(Deserialize, Clone)]
pub struct DiscordInteraction {
    #[serde(rename = "type")]
    pub interaction_type: i8,
    pub data: Option<DiscordInteractionData>,
    pub member: Option<DiscordMember>,
}

#[derive(Deserialize, Clone)]
pub struct DiscordInteractionData {
    pub name: String,
    pub options: Option<Vec<DiscordCommandOption>>,
}

#[derive(Deserialize, Clone)]
pub struct DiscordMember {
    pub user: UserData,
}


#[derive(Deserialize, Clone)]
pub struct UserData {
    pub username: String,
}

#[derive(Deserialize, Clone)]
pub struct DiscordCommandOption {
    pub name: String,
    pub value: Option<Value>
}