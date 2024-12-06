use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::time::Duration;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    // 数字1～10を選択できるセレクトメニューを作成
    let select_menu = CreateSelectMenu::new("number_select", CreateSelectMenuKind::String {
        options: (1..=10)
            .map(|n| CreateSelectMenuOption::new(n.to_string(), n.to_string()))
            .collect(),
    })
    .custom_id("number_select")
    .placeholder("1～10の数字を選んでください");

    // 応答にセレクトメニューを追加
    let _response_message = interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("1～10の数字を選んでください！")
                    .select_menu(select_menu),
            ),
        )
        .await?;

    // メッセージのIDを取得して操作
    let message = interaction.get_response(&ctx).await?;
    
    // 60秒以内に選択を待つ
    let interaction_option = message
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(60))
        .await;

    if let Some(interaction_response) = interaction_option {
        // 選択された数字を取得
        let selected_value = match &interaction_response.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => &values[0],
            _ => return Err(serenity::Error::Other("Unexpected interaction data kind")),
        };

        // DMを送信
        if let Ok(dm_channel) = interaction.user.create_dm_channel(ctx).await {
            dm_channel
                .say(ctx, format!("あなたは **{}** を選びました！", selected_value))
                .await?;
        } else {
            interaction_response
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("DMを送信できませんでした。"),
                    ),
                )
                .await?;
        }

        // ユーザーに応答を送信（セレクトメニューを削除）
        interaction_response
            .create_response(
                ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("DMを送信しました！ご確認ください。"),
                        
                ),
            )
            .await?;
    } else {

    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("select").description("1～10の数字を選択するコマンド")
}
