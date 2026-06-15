use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let message = match interaction.data.resolved.messages.values().next() {
        Some(msg) => msg,
        None => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("メッセージを取得できませんでした。"),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let attachments_info = if message.attachments.is_empty() {
        "なし".to_string()
    } else {
        message
            .attachments
            .iter()
            .map(|a| format!("📎 {} ({} bytes)", a.filename, a.size))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let content = format!(
        "**メッセージ情報**\n作成者: {}\n内容: {}\n添付ファイル:\n{}",
        message.author.name, message.content, attachments_info,
    );

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("メッセージ情報")
        .kind(CommandType::Message)
}
