mod commands;

use dotenv::dotenv;
use std::env;

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::all::Command;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "select" => {
                    if let Err(why) = commands::select::run(&ctx, &command).await {
                        println!("Error executing select command: {why}");
                    }
                }
                "メッセージ情報" => {
                    if let Err(why) = commands::context_menu::run(&ctx, &command).await {
                        println!("Error executing context menu command: {why}");
                    }
                }
                "段階選択" => {
                    if let Err(why) = commands::multi_select::run(&ctx, &command).await {
                        println!("Error executing multi select command: {why}");
                    }
                }
                _ => {
                    let data = CreateInteractionResponseMessage::new()
                        .content("Command not implemented yet.");
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(why) = command.create_response(&ctx.http, builder).await {
                        println!("Error responding to unimplemented command: {why}");
                    }
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let commands = vec![
            commands::select::register(),
            commands::context_menu::register(),
            commands::multi_select::register(),
        ];

        if let Ok(guild_id) = env::var("GUILD_ID") {
            if let Ok(guild_id) = guild_id.parse::<u64>() {
                let guild_id = GuildId::new(guild_id);
                if let Err(why) = guild_id.set_commands(&ctx.http, commands).await {
                    println!("Error registering guild commands: {why}");
                } else {
                    println!("Registered commands for guild {guild_id}");
                }
            } else {
                println!("Warning: GUILD_ID is not a valid integer");
            }
        } else {
            println!("GUILD_ID not set, registering global commands (may take up to 1 hour to propagate)");
            if let Err(why) = Command::set_global_commands(&ctx.http, commands).await {
                println!("Error registering global commands: {why}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Define the necessary intents for the bot
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Initialize the client with the token and intents
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Start the client and handle errors
    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}
