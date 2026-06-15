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

// ==================== FFmpeg共通 ====================

const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mov", "avi", "mkv", "flv", "m4v"];
const AUDIO_EXTS: &[&str] = &["mp3", "aac", "flac", "opus", "wav", "ogg", "m4a", "wma"];

fn is_video_ext(ext: &str) -> bool {
    VIDEO_EXTS.contains(&ext.to_lowercase().as_str())
}

fn is_audio_ext(ext: &str) -> bool {
    AUDIO_EXTS.contains(&ext.to_lowercase().as_str())
}

fn ffmpeg_codec_args(target_ext: &str) -> Vec<&'static str> {
    match target_ext {
        "mp4" => vec!["-c:v", "libx264", "-preset", "medium", "-crf", "23", "-c:a", "aac", "-movflags", "+faststart"],
        "webm" => vec!["-c:v", "libvpx-vp9", "-crf", "30", "-b:v", "0", "-c:a", "libopus"],
        "gif" => vec!["-vf", "fps=10,scale=480:-1:flags=lanczos"],
        "mov" => vec!["-c:v", "libx264", "-preset", "medium", "-crf", "23", "-c:a", "aac"],
        "avi" => vec!["-c:v", "mpeg4", "-q:v", "5", "-c:a", "mp3"],
        "mp3" => vec!["-c:a", "libmp3lame", "-q:a", "2"],
        "aac" => vec!["-c:a", "aac", "-b:a", "192k"],
        "flac" => vec!["-c:a", "flac"],
        "opus" => vec!["-c:a", "libopus", "-b:a", "128k"],
        "wav" => vec!["-c:a", "pcm_s16le"],
        "ogg" => vec!["-c:a", "libvorbis", "-q:a", "5"],
        _ => vec![],
    }
}

async fn run_ffmpeg(args: &[String]) -> Result<(), serenity::Error> {
    println!("[CONV] Running ffmpeg: ffmpeg {}", args.join(" "));
    let status = tokio::process::Command::new("ffmpeg")
        .args(args)
        .status()
        .await
        .map_err(|e| { println!("[CONV] ffmpeg error: {e}"); serenity::Error::Other("ffmpeg not found") })?;

    if !status.success() {
        println!("[CONV] ffmpeg failed with status: {status}");
        return Err(serenity::Error::Other("ffmpeg conversion failed"));
    }
    println!("[CONV] ffmpeg completed successfully");
    Ok(())
}

// ==================== Media targets ====================

fn get_video_targets(ext: &str) -> Vec<(&'static str, &'static str)> {
    let ext = ext.to_lowercase();
    let mut targets = Vec::new();
    for &(e, label) in &[
        ("mp4", "MP4 (H.264)"),
        ("webm", "WebM (VP9)"),
        ("mov", "MOV"),
        ("gif", "GIF"),
        ("avi", "AVI"),
    ] {
        if e != ext {
            targets.push((e, label));
        }
    }
    for &(e, label) in &[
        ("mp3", "MP3"),
        ("aac", "AAC"),
        ("flac", "FLAC"),
        ("opus", "OPUS"),
        ("wav", "WAV"),
        ("ogg", "OGG"),
    ] {
        targets.push((e, label));
    }
    targets
}

fn get_audio_targets(ext: &str) -> Vec<(&'static str, &'static str)> {
    let ext = ext.to_lowercase();
    let mut targets = Vec::new();
    for &(e, label) in &[
        ("mp3", "MP3"),
        ("aac", "AAC"),
        ("flac", "FLAC"),
        ("opus", "OPUS"),
        ("wav", "WAV"),
        ("ogg", "OGG"),
    ] {
        if e != ext {
            targets.push((e, label));
        }
    }
    for &(e, label) in &[
        ("mp4", "MP4 (H.264)"),
        ("webm", "WebM (VP9)"),
    ] {
        targets.push((e, label));
    }
    targets
}

// ==================== FFmpeg routing ====================

fn build_ffmpeg_args(source_ext: &str, target_ext: &str) -> Vec<String> {
    let is_audio_source = is_audio_ext(source_ext);
    let is_video_target = !is_audio_ext(target_ext) && target_ext != "gif";

    let mut args = vec!["-y".to_string()];

    if is_audio_source && is_video_target {
        // Audio → Video: add blank video stream
        args.extend_from_slice(&[
            "-i".into(), format!("input.{source_ext}"),
            "-f".into(), "lavfi".into(),
            "-i".into(), "color=c=black:s=640x360:r=1".into(),
            "-shortest".into(),
        ]);
    } else {
        args.extend_from_slice(&[
            "-i".into(), format!("input.{source_ext}"),
        ]);
    }

    if !is_audio_source && is_audio_ext(target_ext) {
        // Video → Audio: strip video
        args.push("-vn".into());
    }

    args.extend(ffmpeg_codec_args(target_ext).into_iter().map(String::from));
    args.push(format!("output.{target_ext}"));

    args
}

async fn process_ffmpeg_generic(
    ctx: &Context,
    interaction: &ComponentInteraction,
    attachment: &Attachment,
    source_ext: &str,
    target_ext: &str,
) -> Result<(), serenity::Error> {
    println!("[CONV] Downloading from: {}", attachment.url);
    let resp = reqwest::get(&attachment.url)
        .await
        .map_err(|e| { println!("[CONV] Download error: {e}"); serenity::Error::Other("Download error") })?;
    let bytes = resp.bytes().await
        .map_err(|e| { println!("[CONV] Read error: {e}"); serenity::Error::Other("Read error") })?
        .to_vec();
    println!("[CONV] Downloaded {} bytes", bytes.len());

    let in_file = NamedTempFile::with_suffix(&format!(".{source_ext}"))
        .map_err(|e| { println!("[CONV] Temp file error: {e}"); serenity::Error::Other("Temp file error") })?;
    let in_path = in_file.path().to_str().unwrap().to_string();
    std::fs::write(&in_path, &bytes)
        .map_err(|e| { println!("[CONV] Write error: {e}"); serenity::Error::Other("Write error") })?;
    println!("[CONV] Saved to temp file: {in_path}");

    let out_file = NamedTempFile::with_suffix(&format!(".{target_ext}"))
        .map_err(|e| { println!("[CONV] Temp file error: {e}"); serenity::Error::Other("Temp file error") })?;
    let out_path = out_file.path().to_str().unwrap().to_string();

    let raw_args = build_ffmpeg_args(source_ext, target_ext);
    let args: Vec<String> = raw_args.iter().map(|s| {
        if s == &format!("input.{source_ext}") { in_path.clone() }
        else if s == &format!("output.{target_ext}") { out_path.clone() }
        else { s.to_string() }
    }).collect();

    run_ffmpeg(&args).await?;

    let output_bytes = std::fs::read(&out_path)
        .map_err(|e| { println!("[CONV] Read output error: {e}"); serenity::Error::Other("Read output error") })?;
    println!("[CONV] Output size: {} bytes", output_bytes.len());

    let filename = format!("converted.{target_ext}");
    let file = CreateAttachment::bytes(output_bytes, filename.clone());

    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);

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

    let is_image = get_image_source_format(&extension).is_some();
    let is_video = is_video_ext(&extension);
    let is_audio = is_audio_ext(&extension);

    if !is_image && !is_video && !is_audio {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content(format!(
                            "対応していないファイル形式です（{extension}）。\n対応: PNG, JPEG, GIF, WebP, BMP, TIFF, ICO / MP4, WebM, MOV, AVI, MKV, FLV / MP3, AAC, FLAC, OPUS, WAV, OGG"
                        )),
                ),
            )
            .await?;
        return Ok(());
    }

    let (options, media_kind): (Vec<CreateSelectMenuOption>, &str) = if is_image {
        let fmt = get_image_source_format(&extension).unwrap();
        let targets = get_image_targets(fmt);
        if targets.is_empty() {
            interaction.create_response(ctx, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().ephemeral(true).content("このファイルの変換先がありません。"),
            )).await?;
            return Ok(());
        }
        let opts: Vec<CreateSelectMenuOption> = targets.iter()
            .map(|(_, name)| CreateSelectMenuOption::new(*name, name.to_lowercase()))
            .collect();
        (opts, "image")
    } else if is_video {
        let targets = get_video_targets(&extension);
        let opts: Vec<CreateSelectMenuOption> = targets.iter()
            .map(|(e, label)| CreateSelectMenuOption::new(*label, e.to_string()))
            .collect();
        (opts, "video")
    } else {
        let targets = get_audio_targets(&extension);
        let opts: Vec<CreateSelectMenuOption> = targets.iter()
            .map(|(e, label)| CreateSelectMenuOption::new(*label, e.to_string()))
            .collect();
        (opts, "audio")
    };

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("変換元: **{}**\n変換先を選んでください", attachment.filename))
                    .select_menu(CreateSelectMenu::new("format_select", CreateSelectMenuKind::String { options }).placeholder("変換先の形式を選んでください")),
            ),
        )
        .await?;

    let msg = interaction.get_response(&ctx).await?;

    let select_interaction = match msg
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(180))
        .await
    {
        Some(i) => i,
        None => {
            interaction.edit_response(ctx, EditInteractionResponse::new().content("タイムアウトしました。")).await?;
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
                CreateInteractionResponseMessage::new().content("変換中...").components(vec![]),
            ),
        )
        .await?;

    match media_kind {
        "image" => {
            let target_format = get_image_source_format(&chosen)
                .ok_or_else(|| serenity::Error::Other("Invalid format"))?;
            process_image(ctx, &select_interaction, &attachment, target_format).await
        }
        _ => {
            process_ffmpeg_generic(ctx, &select_interaction, &attachment, &extension, &chosen).await
        }
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ファイル変換")
        .kind(CommandType::Message)
}
