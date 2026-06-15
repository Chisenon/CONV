use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::time::Duration;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let step1_menu = CreateSelectMenu::new(
        "step1",
        CreateSelectMenuKind::String {
            options: vec![
                CreateSelectMenuOption::new("選択肢 1", "1"),
                CreateSelectMenuOption::new("選択肢 2", "2"),
            ],
        },
    )
    .placeholder("1か2を選んでください");

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("**Step 1**: 1か2を選んでください")
                    .select_menu(step1_menu),
            ),
        )
        .await?;

    let message = interaction.get_response(&ctx).await?;

    let step1_interaction = match message
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(i) => i,
        None => {
            interaction
                .edit_response(ctx, EditInteractionResponse::new()
                    .content("タイムアウトしました。"))
                .await?;
            return Ok(());
        }
    };

    let step1_value = match &step1_interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].clone(),
        _ => return Err(serenity::Error::Other("Unexpected interaction data")),
    };

    let step2_options = match step1_value.as_str() {
        "1" => vec![
            CreateSelectMenuOption::new("選択肢 A", "A"),
            CreateSelectMenuOption::new("選択肢 B", "B"),
        ],
        "2" => vec![
            CreateSelectMenuOption::new("選択肢 C", "C"),
            CreateSelectMenuOption::new("選択肢 D", "D"),
        ],
        _ => return Err(serenity::Error::Other("Invalid selection")),
    };

    let step2_menu = CreateSelectMenu::new(
        "step2",
        CreateSelectMenuKind::String {
            options: step2_options,
        },
    )
    .placeholder("次の選択肢を選んでください");

    step1_interaction
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("**Step 2**: 次の選択肢を選んでください")
                    .select_menu(step2_menu),
            ),
        )
        .await?;

    let step2_message = interaction.get_response(&ctx).await?;

    let step2_interaction = match step2_message
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(i) => i,
        None => {
            interaction
                .edit_response(ctx, EditInteractionResponse::new()
                    .content("タイムアウトしました。"))
                .await?;
            return Ok(());
        }
    };

    let step2_value = match &step2_interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].clone(),
        _ => return Err(serenity::Error::Other("Unexpected interaction data")),
    };

    step2_interaction
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "完了！選んだのは **{} → {}** です。",
                        step1_value, step2_value,
                    ))
                    .components(vec![]),
            ),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("段階選択")
        .kind(CommandType::Message)
}
