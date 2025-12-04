use anyhow::Result;
use base64::Engine;
use clap::{CommandFactory, Parser};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::io::{self, IsTerminal, Read};
use log::error;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "asr",
    author,
    version,
    about = "Qwen3 realtime ASR, Unix philosophy: read audio from stdin, output JSON events to stdout",
    long_about = r#"Input audio: PCM s16le (16-bit signed little-endian), mono

Usage examples (ffmpeg -> stdin):
  macOS (AVFoundation):
    ffmpeg -f avfoundation -i ":0" -f s16le -ar 16000 -ac 1 - 2>/dev/null | asr

  Linux (ALSA):
    ffmpeg -f alsa -i default -f s16le -ar 16000 -ac 1 - 2>/dev/null | asr

  Windows (DirectShow):
    ffmpeg -f dshow -i audio="Microphone" -f s16le -ar 16000 -ac 1 - 2>/dev/null | asr

Environment:
  - Set DASHSCOPE_API_KEY via env or use --api-key
"#
)]
struct Args {
    #[arg(long, env = "DASHSCOPE_API_KEY")]
    api_key: String,
    #[arg(long, short, default_value = "qwen3-asr-flash-realtime")]
    model: String,
    #[arg(long, default_value = "wss://dashscope.aliyuncs.com/api-ws/v1/realtime")]
    base_url: String,
    #[arg(long, short, default_value_t = 16000)]
    sample_rate: u32,
    #[arg(long, short, default_value = "zh")]
    language: String,
    #[arg(long, default_value_t = 0.2)]
    vad_threshold: f32,
    #[arg(long, default_value_t = 800)]
    vad_silence_ms: u32,
    #[arg(short, long)]
    keep: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    if io::stdin().is_terminal() {
        Args::command().print_help()?;
        std::process::exit(0);
    }
    
    let url = format!("{}?model={}", args.base_url, args.model);

    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(&url)
        .header("Authorization", format!("Bearer {}", args.api_key))
        .header("OpenAI-Beta", "realtime=v1")
        .header("Host", "dashscope.aliyuncs.com")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .body(())?;

    let (ws_stream, _) = connect_async(request).await?;
    let (mut message_tx, mut message_rx) = ws_stream.split();

    let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(128);

    // Send session configuration
    let session_update = json!({
        "event_id": Uuid::now_v7().to_string(),
        "type": "session.update",
        "session": {
            "modalities": ["text"],
            "input_audio_format": "pcm",
            "sample_rate": args.sample_rate,
            "input_audio_transcription": {
                "language": args.language
            },
            "turn_detection": {
                "type": "server_vad",
                "threshold": args.vad_threshold,
                "silence_duration_ms": args.vad_silence_ms
            }
        }
    });

    message_tx.send(Message::Text(session_update.to_string().into())).await?;

    tokio::task::spawn_blocking(move || {
        if let Err(err) = read_audio_data(audio_tx) {
            error!("Error reading stdin: {err}");
        }
    });

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let keep = args.keep;
    
    let _task_w_audio = tokio::spawn(async move {
        while let Some(audio_data) = audio_rx.recv().await {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&audio_data);
            let audio_event = json!({
                "event_id": Uuid::now_v7().to_string(),
                "type": "input_audio_buffer.append",
                "audio": encoded
            });

            if message_tx.send(Message::Text(audio_event.to_string().into())).await.is_err() {
                error!("Failed to send audio data");
                break;
            }
        }
        
        if !keep {
            let _ = shutdown_tx.send(());
        }
    });
    
    let task_r_message = tokio::spawn(async move {
        while let Some(msg) = message_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("{text}");
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(err) => {
                    error!("Error receiving message: {err}");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = task_r_message => {},
        _ = tokio::signal::ctrl_c() => {},
        _ = shutdown_rx => {}
    }

    Ok(())
}

fn read_audio_data(audio_tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
    let mut stdin = io::stdin();
    let mut buffer = [0u8; 8192];

    loop {
        let n = stdin.read(&mut buffer)?;

        if n == 0 {
            break
        }

        audio_tx.blocking_send(buffer[..n].to_vec())?;
    }

    Ok(())
}