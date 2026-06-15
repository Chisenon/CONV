mod commands;

use dotenv::dotenv;
use std::env;

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::prelude::Command;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "ファイル変換" => {
                    if let Err(why) = commands::convert::run(&ctx, &command).await {
                        println!("Error executing convert command: {why}");
                    }
                }
                _ => {}
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let commands = vec![
            commands::convert::register(),
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
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}