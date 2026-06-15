use image::ImageFormat;
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::io::Cursor;
use std::time::Duration;
use tempfile::NamedTempFile;

// ==================== Image ====================

fn get_image_source_format(ext: &str) -> Option<ImageFormat> {
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

fn get_image_targets(source: ImageFormat) -> Vec<(ImageFormat, &'static str)> {
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

fn image_format_extension(fmt: ImageFormat) -> &'static str {
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

async fn process_image(
    ctx: &Context,
    interaction: &ComponentInteraction,
    attachment: &Attachment,
    target_format: ImageFormat,
) -> Result<(), serenity::Error> {
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

    let source_format = get_image_source_format(
        attachment.filename.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
    ).unwrap_or(ImageFormat::Png);

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

    let ext = image_format_extension(target_format);
    let filename = format!("converted.{}", ext);
    let file = CreateAttachment::bytes(output.into_inner(), filename.clone());

    send_result(ctx, interaction, attachment, &filename, file).await
}

// ==================== Video ====================

const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mov", "avi", "mkv", "flv", "m4v"];

fn get_video_targets(ext: &str) -> Vec<(&'static str, &'static str)> {
    let ext = ext.to_lowercase();
    let mut targets = Vec::new();
    for &(e, label) in &[
        ("mp4", "MP4 (H.264)"),
        ("webm", "WebM (VP9)"),
        ("mov", "MOV"),
        ("gif", "GIF (アニメーション)"),
        ("avi", "AVI"),
    ] {
        if e != ext {
            targets.push((e, label));
        }
    }
    targets
}

fn ffmpeg_args(input: &str, output: &str, target_ext: &str) -> Vec<String> {
    let mut args = vec!["-y".to_string(), "-i".to_string(), input.to_string()];
    match target_ext {
        "mp4" => args.extend_from_slice(&[
            "-c:v".into(), "libx264".into(),
            "-preset".into(), "medium".into(),
            "-crf".into(), "23".into(),
            "-c:a".into(), "aac".into(),
            "-movflags".into(), "+faststart".into(),
        ]),
        "webm" => args.extend_from_slice(&[
            "-c:v".into(), "libvpx-vp9".into(),
            "-crf".into(), "30".into(),
            "-b:v".into(), "0".into(),
            "-c:a".into(), "libopus".into(),
        ]),
        "gif" => args.extend_from_slice(&[
            "-vf".into(), "fps=10,scale=480:-1:flags=lanczos".into(),
        ]),
        "mov" => args.extend_from_slice(&[
            "-c:v".into(), "libx264".into(),
            "-preset".into(), "medium".into(),
            "-crf".into(), "23".into(),
            "-c:a".into(), "aac".into(),
        ]),
        "avi" => args.extend_from_slice(&[
            "-c:v".into(), "mpeg4".into(),
            "-q:v".into(), "5".into(),
            "-c:a".into(), "mp3".into(),
        ]),
        _ => {}
    }
    args.push(output.to_string());
    args
}

async fn process_video(
    ctx: &Context,
    interaction: &ComponentInteraction,
    attachment: &Attachment,
    target_ext: &str,
) -> Result<(), serenity::Error> {
    println!("[CONV] Downloading video from: {}", attachment.url);
    let resp = reqwest::get(&attachment.url)
        .await
        .map_err(|e| { println!("[CONV] Download error: {e}"); serenity::Error::Other("Download error") })?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| { println!("[CONV] Read error: {e}"); serenity::Error::Other("Read error") })?
        .to_vec();
    println!("[CONV] Downloaded {} bytes", bytes.len());

    let source_ext = attachment.filename.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    let input_file = NamedTempFile::with_suffix(&format!(".{source_ext}"))
        .map_err(|e| { println!("[CONV] Temp file error: {e}"); serenity::Error::Other("Temp file error") })?;
    let input_path = input_file.path().to_str().unwrap().to_string();

    let output_file = NamedTempFile::with_suffix(&format!(".{target_ext}"))
        .map_err(|e| { println!("[CONV] Temp file error: {e}"); serenity::Error::Other("Temp file error") })?;
    let output_path = output_file.path().to_str().unwrap().to_string();

    std::fs::write(&input_path, &bytes)
        .map_err(|e| { println!("[CONV] Write error: {e}"); serenity::Error::Other("Write error") })?;
    println!("[CONV] Saved to temp file: {input_path}");

    let args = ffmpeg_args(&input_path, &output_path, target_ext);
    println!("[CONV] Running ffmpeg: ffmpeg {}", args.join(" "));

    let status = tokio::process::Command::new("ffmpeg")
        .args(&args)
        .status()
        .await
        .map_err(|e| { println!("[CONV] ffmpeg error: {e}"); serenity::Error::Other("ffmpeg error") })?;

    if !status.success() {
        println!("[CONV] ffmpeg failed with status: {status}");
        return Err(serenity::Error::Other("ffmpeg conversion failed"));
    }
    println!("[CONV] ffmpeg completed successfully");

    let output_bytes = std::fs::read(&output_path)
        .map_err(|e| { println!("[CONV] Read output error: {e}"); serenity::Error::Other("Read output error") })?;
    println!("[CONV] Output size: {} bytes", output_bytes.len());

    let filename = format!("converted.{target_ext}");
    let file = CreateAttachment::bytes(output_bytes, filename.clone());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    send_result(ctx, interaction, attachment, &filename, file).await
}

// ==================== Common ====================

async fn send_result(
    ctx: &Context,
    interaction: &ComponentInteraction,
    attachment: &Attachment,
    filename: &str,
    file: CreateAttachment,
) -> Result<(), serenity::Error> {
    interaction
        .edit_response(ctx, EditInteractionResponse::new().content("変換完了！"))
        .await
        .ok();

    let result = interaction
        .create_followup(
            ctx,
            CreateInteractionResponseFollowup::new()
                .content(format!("変換完了！（{} → {}）", attachment.filename, filename))
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

// ==================== Entry point ====================

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
        a.filename.rsplit_once('.').map(|(_, e)| e.to_lowercase()).is_some()
    }) {
        Some(a) => a.clone(),
        None => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("ファイルがありません。"),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let extension = match attachment.filename.rsplit_once('.') {
        Some((_, ext)) => ext.to_lowercase(),
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

    // Determine media type and build select menu options
    let is_image = get_image_source_format(&extension).is_some();
    let is_video = VIDEO_EXTS.contains(&extension.as_str());

    if !is_image && !is_video {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content(format!(
                            "対応していないファイル形式です（{extension}）。\n対応画像: PNG, JPEG, GIF, WebP, BMP, TIFF, ICO\n対応動画: MP4, WebM, MOV, AVI, MKV, FLV"
                        )),
                ),
            )
            .await?;
        return Ok(());
    }

    // Build select menu options
    let (options, is_image_targets): (Vec<CreateSelectMenuOption>, bool) = if is_image {
        let fmt = get_image_source_format(&extension).unwrap();
        let targets = get_image_targets(fmt);
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
        let opts: Vec<CreateSelectMenuOption> = targets
            .iter()
            .map(|(_, name)| CreateSelectMenuOption::new(*name, name.to_lowercase()))
            .collect();
        (opts, true)
    } else {
        let targets = get_video_targets(&extension);
        let opts: Vec<CreateSelectMenuOption> = targets
            .iter()
            .map(|(ext, label)| CreateSelectMenuOption::new(*label, ext.to_string()))
            .collect();
        (opts, false)
    };

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
        .timeout(Duration::from_secs(120))
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

    println!("[CONV] Starting conversion: {} -> {}", attachment.filename, chosen);

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

    if is_image_targets {
        let target_format = get_image_source_format(&chosen)
            .ok_or_else(|| serenity::Error::Other("Invalid format"))?;
        process_image(ctx, &select_interaction, &attachment, target_format).await
    } else {
        process_video(ctx, &select_interaction, &attachment, &chosen).await
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ファイル変換")
        .kind(CommandType::Message)
}
