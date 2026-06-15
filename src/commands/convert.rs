use image::ImageFormat;
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::io::Cursor;
use std::time::Duration;

fn get_source_format(ext: &str) -> Option<ImageFormat> {
    match ext.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "gif" => Some(ImageFormat::Gif),
        "webp" => Some(ImageFormat::WebP),
        "bmp" => Some(ImageFormat::Bmp),
        "tiff" | "tif" => Some(ImageFormat::Tiff),
        "ico" => Some(ImageFormat::Ico),
        _ => None,
    }
}

fn get_targets(source: ImageFormat) -> Vec<(ImageFormat, &'static str)> {
    use ImageFormat::*;
    match source {
        Png => vec![(Jpeg, "JPEG"), (WebP, "WebP"), (Gif, "GIF"), (Bmp, "BMP"), (Ico, "ICO"), (Tiff, "TIFF")],
        Jpeg => vec![(Png, "PNG"), (WebP, "WebP"), (Gif, "GIF"), (Bmp, "BMP"), (Tiff, "TIFF")],
        Gif => vec![(Png, "PNG"), (Jpeg, "JPEG"), (WebP, "WebP"), (Bmp, "BMP")],
        WebP => vec![(Png, "PNG"), (Jpeg, "JPEG"), (Gif, "GIF"), (Bmp, "BMP")],
        Bmp => vec![(Png, "PNG"), (Jpeg, "JPEG"), (WebP, "WebP"), (Gif, "GIF")],
        Tiff => vec![(Png, "PNG"), (Jpeg, "JPEG"), (WebP, "WebP")],
        Ico => vec![(Png, "PNG"), (Jpeg, "JPEG"), (WebP, "WebP")],
        _ => vec![],
    }
}

fn format_extension(fmt: ImageFormat) -> &'static str {
    use ImageFormat::*;
    match fmt {
        Png => "png",
        Jpeg => "jpg",
        Gif => "gif",
        WebP => "webp",
        Bmp => "bmp",
        Tiff => "tiff",
        Ico => "ico",
        _ => "bin",
    }
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let resolved = match interaction.data.resolved.messages.values().next() {
        Some(msg) => msg.clone(),
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

    let attachment = match resolved.attachments.iter().find(|a| {
        let ext = a.filename.rsplit_once('.').map(|(_, e)| e.to_lowercase());
        matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "ico"))
    }) {
        Some(a) => a.clone(),
        None => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("このメッセージに変換可能な画像ファイルがありません。\n対応: PNG, JPEG, GIF, WebP, BMP, TIFF, ICO"),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let extension = match attachment.filename.rsplit_once('.') {
        Some((_, ext)) => ext.to_string(),
        None => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("ファイルの拡張子が判別できません。"),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let source_format = match get_source_format(&extension) {
        Some(f) => f,
        None => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content(format!(
                                "対応していないファイル形式です（{}）。\n対応: PNG, JPEG, GIF, WebP, BMP, TIFF, ICO",
                                extension
                            )),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let targets = get_targets(source_format);
    if targets.is_empty() {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("このファイルの変換先がありません。"),
                ),
            )
            .await?;
        return Ok(());
    }

    let options: Vec<CreateSelectMenuOption> = targets
        .iter()
        .map(|(_, name)| CreateSelectMenuOption::new(*name, name.to_lowercase()))
        .collect();

    let select_menu = CreateSelectMenu::new("format_select", CreateSelectMenuKind::String { options })
        .placeholder("変換先の形式を選んでください");

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("変換元: **{}**\n変換先を選んでください", attachment.filename))
                    .select_menu(select_menu),
            ),
        )
        .await?;

    let msg = interaction.get_response(&ctx).await?;

    let select_interaction = match msg
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(i) => i,
        None => {
            interaction
                .edit_response(ctx, EditInteractionResponse::new().content("タイムアウトしました。"))
                .await?;
            return Ok(());
        }
    };

    let chosen = match &select_interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].clone(),
        _ => return Err(serenity::Error::Other("Unexpected interaction data")),
    };

    let target_format = match targets.iter().find(|(_, name)| name.to_lowercase() == chosen.as_str()) {
        Some((fmt, _)) => *fmt,
        None => return Err(serenity::Error::Other("Invalid format")),
    };

    println!("[CONV] Starting conversion: {} -> {:?}", attachment.filename, target_format);

    select_interaction
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("変換中...")
                    .components(vec![]),
            ),
        )
        .await?;

    println!("[CONV] Downloading from: {}", attachment.url);
    let resp = reqwest::get(&attachment.url)
        .await
        .map_err(|e| { println!("[CONV] Download error: {e}"); serenity::Error::Other("Download error") })?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| { println!("[CONV] Read error: {e}"); serenity::Error::Other("Read error") })?
        .to_vec();
    println!("[CONV] Downloaded {} bytes", bytes.len());

    let mut img = image::load(Cursor::new(bytes), source_format)
        .map_err(|e| { println!("[CONV] Image load error: {e}"); serenity::Error::Other("Image load error") })?;
    println!("[CONV] Image loaded successfully");

    if target_format == ImageFormat::Ico && (img.width() > 256 || img.height() > 256) {
        let ratio = 256.0 / img.width().max(img.height()) as f64;
        let w = (img.width() as f64 * ratio) as u32;
        let h = (img.height() as f64 * ratio) as u32;
        img = img.resize_exact(w.max(1), h.max(1), image::imageops::FilterType::Lanczos3);
        println!("[CONV] Resized to {w}x{h} for ICO format");
    }

    let mut output = Cursor::new(Vec::new());
    img.write_to(&mut output, target_format)
        .map_err(|e| { println!("[CONV] Image write error: {e}"); serenity::Error::Other("Image write error") })?;
    println!("[CONV] Conversion completed");

    let ext = format_extension(target_format);
    let filename = format!("converted.{}", ext);
    let file = CreateAttachment::bytes(output.into_inner(), filename.clone());
    println!("[CONV] Sending file: {}", filename);

    select_interaction
        .edit_response(ctx, EditInteractionResponse::new().content("変換完了！"))
        .await
        .ok();

    let result = select_interaction
        .create_followup(
            ctx,
            CreateInteractionResponseFollowup::new()
                .content(format!("画像変換完了！（{} → .{}）", attachment.filename, ext))
                .add_file(file)
                .ephemeral(true),
        )
        .await;

    match result {
        Ok(_) => println!("[CONV] Followup sent successfully"),
        Err(e) => println!("[CONV] Followup error: {e}"),
    }

    println!("[CONV] Done");

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("画像変換")
        .kind(CommandType::Message)
}
