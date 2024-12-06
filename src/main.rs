mod commands;

use dotenv::dotenv;
use std::env;

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "select" => match commands::select::run(&ctx, &command).await {
                    Ok(_) => {
                        let data = CreateInteractionResponseMessage::new()
                            .content("Select command executed successfully.");
                        let builder = CreateInteractionResponse::Message(data);
                        if let Err(why) = command.create_response(&ctx.http, builder).await {
                            println!("Error responding to select command: {why}");
                        }
                    }
                    Err(why) => {
                        let data = CreateInteractionResponseMessage::new()
                            .content(format!("Error executing select command: {why}"));
                        let builder = CreateInteractionResponse::Message(data);
                        if let Err(why) = command.create_response(&ctx.http, builder).await {
                            println!("Error responding to select command error: {why}");
                        }
                    }
                },
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

        // Fetching the guild ID from environment variables
        let guild_id = GuildId::new(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        // Registering commands
        if let Err(why) = guild_id
            .set_commands(
                &ctx.http,
                vec![commands::select::register()],
            )
            .await
        {
            println!("Error registering commands: {why}");
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
