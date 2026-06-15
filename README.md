# CONV

右クリックメニューから画像・動画を変換するDiscord Bot。

## できること

メッセージ内のファイルを右クリック → 「ファイル変換」→ 変換先を選択 → 変換結果がephemeralで届く。

### 画像変換

| 変換元 | 変換先 |
|--------|--------|
| PNG    | JPEG, WebP, GIF, BMP, ICO, TIFF |
| JPEG   | PNG, WebP, GIF, BMP, TIFF |
| GIF    | PNG, JPEG, WebP, BMP |
| WebP   | PNG, JPEG, GIF, BMP |
| BMP    | PNG, JPEG, WebP, GIF |
| TIFF   | PNG, JPEG, WebP |
| ICO    | PNG, JPEG, WebP |

- ICO変換時は256×256に自動リサイズ
- アニメーションGIFは先頭フレームのみ変換

### 動画変換（ffmpeg）

| 変換元 | 変換先 |
|--------|--------|
| MP4    | MP4 (H.264), WebM (VP9), MOV, GIF, AVI |
| WebM   | MP4 (H.264), MOV, GIF, AVI |
| MOV    | MP4 (H.264), WebM (VP9), GIF, AVI |
| AVI    | MP4 (H.264), WebM (VP9), MOV, GIF |
| MKV    | MP4 (H.264), WebM (VP9), MOV, GIF, AVI |
| FLV    | MP4 (H.264), WebM (VP9), MOV, GIF, AVI |

- ffmpegがサーバーにインストールされている必要があります
- GIF出力はfps=10,幅480pxに自動調整

### 共通

- 変換後ファイルはephemeralメッセージで届く（本人のみ閲覧可）

## セットアップ

1. `.env.sample` を `.env` にコピーして編集
2. 以下の内容を記述

```
DISCORD_TOKEN="your_discord_token"
GUILD_ID="your_guild_id"
```

`GUILD_ID` は省略可（省略時はグローバルコマンド登録、反映に最大1時間）

3. `cargo run` で起動

## 依存

- serenity 0.12.5
- image 0.25.5
- reqwest 0.12.9（rustls-tls）
- tempfile 3.15.0
- tokio
- ffmpeg（動画変換時、システムにインストール必須）
