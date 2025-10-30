use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{self, Command};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, thread};

use rust_logger::{Logger, Severity};

use crate::components::{Piper, config_reader};
mod components;

#[allow(dead_code)]
#[derive(Debug)]
struct IrcMessage {
    sender: String,
    channel: String,
    message: String,
}

fn convert_audio(audio_path: &str, output: &str) {
    Command::new("ffmpeg")
        .args([
            "-f",
            "f32le",
            "-ar",
            "22050",
            "-ac",
            "1",
            "-i",
            audio_path,
            "-c:a",
            "pcm_s16le",
            "-y",
            output,
        ])
        .output()
        .expect("ffmpeg failed");
}

fn play_audio(audio_path: &str) {
    Command::new("ffplay")
        .args(["-nodisp", "-autoexit", audio_path])
        .output()
        .expect("ffplay failed");
}

fn parse_irc_line(line: &str) -> Option<IrcMessage> {
    // Only handle PRIVMSG lines
    if !line.contains("PRIVMSG") {
        return None;
    }

    // Example line:
    // :sky_preacherman!sky_preacherman@sky_preacherman.tmi.twitch.tv PRIVMSG #hollsbeauti :I was on an app...

    // Split into 3 parts: prefix, command, trailing
    let mut parts = line.splitn(3, ' ');

    let prefix = parts.next()?; // ":sky_preacherman!sky_preacherman@..."
    let _command = parts.next()?; // "PRIVMSG"
    let rest = parts.next()?; // "#hollsbeauti :I was on ..."

    // Extract channel and message
    let mut rest_parts = rest.splitn(2, " :");
    let channel = rest_parts.next()?.to_string(); // "#hollsbeauti"
    let message = rest_parts.next()?.trim().to_string(); // "I was on an app..."

    // Extract sender name from prefix
    // prefix is like ":sky_preacherman!sky_preacherman@..."
    let sender = prefix
        .trim_start_matches(':')
        .split('!')
        .next()?
        .to_string();

    Some(IrcMessage {
        sender,
        channel,
        message,
    })
}

fn main() -> io::Result<()> {
    let log = Logger::new("Main");
    let twitch_config = config_reader::read_and_parse_config().unwrap_or_else(|| {
        log.error("No valid config found", Severity::Critical);
        process::exit(1);
    });

    // Create channel for messages
    let (tx, rx) = mpsc::channel::<IrcMessage>();

    // Spawn TTS thread that owns Piper
    thread::spawn(move || {
        let tts = Piper::new(
            "./en_US-john-medium.onnx",
            "./en_US-john-medium.onnx.json",
            "./espeak-ng-data",
        );

        while let Ok(irc) = rx.recv() {
            // generate a unique filename based on timestamp
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let safe_output_file_name = irc
                .sender
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(20)
                .collect::<String>();
            let raw_file = format!("{}_{}.raw", safe_output_file_name, timestamp);
            let wav_file = format!("{}_{}.wav", safe_output_file_name, timestamp);

            // generate, convert, and play
            tts.generate(&irc.message, &raw_file);
            convert_audio(&raw_file, &wav_file);
            play_audio(&wav_file);
            let _ = fs::remove_file(raw_file);
            let _ = fs::remove_file(wav_file);
        }
    });

    // Connect to Twitch IRC
    let mut stream = TcpStream::connect("irc.chat.twitch.tv:6667")?;
    log.info("Connected to Twitch IRC!");

    let username = twitch_config.username;
    let oauth_token = format!("oauth:{}", twitch_config.oauth_token);
    let channel = format!("#{}", twitch_config.channel);

    writeln!(stream, "PASS {}", oauth_token)?;
    writeln!(stream, "NICK {}", username)?;
    writeln!(stream, "JOIN {}", channel)?;
    stream.flush()?;

    log.info(format!("Joined {}", channel));

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            log.error("Disconnected", Severity::Critical);
            break;
        }

        if let Some(message) = parse_irc_line(&line) {
            // Send message to TTS thread
            log.info(format!("Reading: {}", message.message));
            tx.send(message).unwrap();
        }

        if line.starts_with("PING") {
            let resp = line.replace("PING", "PONG");
            stream.write_all(resp.as_bytes())?;
            log.info(format!("< {}", resp.trim()));
        }
    }

    Ok(())
}
