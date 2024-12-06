mod commands;

use dotenv::dotenv;
use serenity::Client;
use serde_json::Value;
use std::env;

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::Interaction;
use serenity::model::channel::{Message, Reaction, ReactionType};
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

        let guild_id = GuildId::new(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

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

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let ReactionType::Unicode(unicode) = &reaction.emoji {
            println!("Received unicode reaction: {}", unicode);

            if unicode == "♻️" {
                if let Ok(message) = reaction.message(&ctx.http).await {
                    simulate_command_execution(&ctx, message, "!hi".to_string()).await;
                } else {
                    eprintln!("Failed to fetch the message for reaction ♻️");
                }
            }
        } else if let ReactionType::Custom { id, name, .. } = &reaction.emoji {
            println!("Received custom reaction: ID = {}, name = {:?}", id, name);
        } else {
            println!("Received unknown reaction type.");
        }
    }
}

/// Simulates command execution for bot messages
async fn simulate_command_execution(ctx: &Context, msg: Message, content: String) {
    if let Some(command) = content.strip_prefix('!') {
        handle_command(ctx, &msg, command).await;
    }
}

/// Handles a given command
async fn handle_command(ctx: &Context, msg: &Message, command: &str) {
    match command {
        "hi" => {
            if let Err(e) = fetch_and_reply(ctx, msg).await {
                log_error("Error in fetch_message_and_output_details", &e);
            }
        }
        _ => {
            if let Err(e) = msg.reply(ctx, "そのコマンドはわかりません。!hi を試してください。").await {
                log_error("Error sending reply", &e);
            }
        }
    }
}

/// Fetches message details and replies to the user
async fn fetch_and_reply(ctx: &Context, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();

    fetch_message_and_output_details(ctx, &channel_id, &message_id).await?;

    Ok(())
}

/// Fetches message details and processes attachments
async fn fetch_message_and_output_details(
    ctx: &Context,
    channel_id: &str,
    message_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let client = reqwest::Client::new();
    let url = format!(
        "https://discord.com/api/v10/channels/{}/messages/{}",
        channel_id, message_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bot {}", token))
        .send()
        .await?;

    if response.status().is_success() {
        let body: Value = response.json().await?;
        if let Some(attachments) = body.get("attachments").and_then(|a| a.as_array()) {
            if !attachments.is_empty() {
                let reply_content: String = attachments
                    .iter()
                    .map(|attachment| {
                        let filename = attachment.get("filename").unwrap_or(&Value::Null);
                        let url = attachment.get("url").unwrap_or(&Value::Null);
                        format!("ファイル名：{}\nURL：{}\n", filename, url)
                    })
                    .collect();

                send_reply(ctx, channel_id, message_id, reply_content).await?;
            } else {
                println!("添付ファイルはありません");
            }
        }
    } else {
        eprintln!("Failed to fetch message: HTTP {}", response.status());
    }

    Ok(())
}

/// Sends a reply to a message
async fn send_reply(
    ctx: &Context,
    channel_id: &str,
    message_id: &str,
    content: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let channel_id_u64 = channel_id.parse::<u64>()?;
    let message_id_u64 = message_id.parse::<u64>()?;
    let channel = serenity::model::id::ChannelId::from(channel_id_u64);
    let message = channel.message(ctx, message_id_u64).await?;
    message.reply(ctx, content).await?;
    Ok(())
}

/// Logs errors in a standardized way
fn log_error(context: &str, error: &dyn std::fmt::Debug) {
    eprintln!("{}: {:?}", context, error);
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

        let mut client = Client::builder(&token, intents)
        .token(&token)
        .intents(intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}
