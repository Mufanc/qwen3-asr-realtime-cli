# Qwen3 ASR Realtime CLI

A command-line tool for real-time automatic speech recognition using Qwen3 ASR models via WebSocket streaming.

## Overview

This tool follows the Unix philosophy: read PCM audio from stdin, communicate with Qwen3 ASR service over WebSocket, and output JSON events to stdout. It supports server-side voice activity detection (VAD) and real-time transcription.

## Features

- Real-time speech recognition using Qwen3 ASR models
- Server-side voice activity detection
- Streaming audio input via stdin (PCM s16le format)
- JSON event output for easy parsing and integration
- Configurable sample rate, language, and VAD parameters

## Installation

```bash
cargo install --path .
```

## Usage

The tool expects PCM audio input in s16le format (16-bit signed little-endian), mono channel.

### Basic Usage

```bash
ffmpeg -i input.mp3 -f s16le -ar 16000 -ac 1 - | asr --api-key YOUR_API_KEY
```

### Microphone Input (macOS)

```bash
ffmpeg -f avfoundation -i ":0" -f s16le -ar 16000 -ac 1 - | asr --api-key YOUR_API_KEY
```

### Environment Variable

Set your API key as an environment variable:

```bash
export DASHSCOPE_API_KEY=your_api_key_here
asr < audio.pcm
```

## Command-Line Options

| Option | Default | Description |
|--------|---------|-------------|
| `--api-key` | - | DashScope API key (required) |
| `--model`, `-m` | `qwen3-asr-flash-realtime` | ASR model to use |
| `--base-url` | `wss://dashscope.aliyuncs.com/api-ws/v1/realtime` | WebSocket endpoint |
| `--sample-rate`, `-s` | `16000` | Audio sample rate in Hz |
| `--language`, `-l` | `zh` | Recognition language code |
| `--vad-threshold` | `0.2` | Voice activity detection threshold |
| `--vad-silence-ms` | `800` | Silence duration in milliseconds for VAD |

## Output Format

The tool outputs JSON events to stdout. Each line is a JSON object representing an event from the ASR service, such as transcription results, session updates, or errors.

## Requirements

- Rust 1.70 or higher
- FFmpeg (for audio format conversion)
- Valid DashScope API key