mod commands;

use dotenv::dotenv;
use std::env;

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
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

        // Registering global commands
        if let Err(why) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            vec![commands::select::register()],
        )
        .await
        {
            println!("Error registering global commands: {why}");
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