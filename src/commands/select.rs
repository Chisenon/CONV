use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::time::Duration;
use log::error;  // ログ出力を使用

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    // 数字1～10を選択できるセレクトメニューを作成
    let select_menu = CreateSelectMenu::new(
        "number_select",
        CreateSelectMenuKind::String {
            options: (1..=10)
                .map(|n| CreateSelectMenuOption::new(n.to_string(), n.to_string()))
                .collect(),
        },
    )
    .custom_id("number_select")
    .placeholder("1～10の数字を選んでください");

    // 応答にセレクトメニューを追加
    interaction
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

    let interaction_option = message
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(20))
        .await;

    // 20秒以内に選択を待つ
    if let Some(interaction_response) = interaction_option {
        // 選択された数字を取得
        let selected_value = match &interaction_response.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => &values[0],
            _ => {
                return Err(serenity::Error::Other("Unexpected interaction data kind"));
            }
        };

        // DMを送信
        if let Ok(dm_channel) = interaction.user.create_dm_channel(ctx).await {
            // DM送信成功時
            if let Err(e) = dm_channel
                .say(
                    ctx,
                    format!("あなたは **{}** を選びました！", selected_value),
                )
                .await
            {
                // DM送信失敗時のログ出力
                error!("DM送信失敗: {}", e);
                interaction_response
                    .create_response(
                        ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content("DMが送信できませんでした。")
                                .components(vec![]),
                        ),
                    )
                    .await?;
            } else {
                // DM送信成功後のレスポンス
                interaction_response
                    .create_response(
                        ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content("DMを送信しました！ご確認ください。")
                                .components(vec![]),
                        ),
                    )
                    .await?;
            }
        }
    } else {
        // タイムアウト処理
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("タイムアウトしました。もう一度試してください。")
                        .components(vec![]),
                ),
            )
            .await?;
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("select").description("1～10の数字を選択するコマンド")
}
