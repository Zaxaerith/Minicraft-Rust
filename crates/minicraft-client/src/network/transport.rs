//! Line-delimited TCP transport for the shared multiplayer protocol.
//!
//! Java's `network/` package only names `Network` and `MinicraftProtocol`;
//! the socket client itself is the Rust probe surface. LOGIN is always the
//! first frame; PING/PONG is the heartbeat (Java `InputType.PING`).
use minicraft_common::protocol::{
    GAME_VERSION, Message, PROTOCOL_VERSION, decode_line, encode_line,
};
use std::{
    io::{BufReader, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

pub fn run_client_probe(address: &str, username: &str) -> Result<(), String> {
    let mut client = MultiplayerClient::connect(address, username)?;
    client.send_player(0, 0, 1, 10)?;
    let nonce = 0x224;
    client.ping(nonce)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if client
            .poll()
            .into_iter()
            .any(|message| message == Message::Pong { nonce })
        {
            println!(
                "Connected to {address} as {username} (client {})",
                client.client_id
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(2));
    }
    Err("multiplayer probe timed out waiting for PONG".to_owned())
}

pub struct MultiplayerClient {
    pub client_id: u64,
    writer: TcpStream,
    receiver: Receiver<Message>,
    reader: Option<thread::JoinHandle<()>>,
}

impl MultiplayerClient {
    pub fn connect(address: &str, username: &str) -> Result<Self, String> {
        let mut connected = None;
        for endpoint in address
            .to_socket_addrs()
            .map_err(|error| error.to_string())?
        {
            if let Ok(stream) = TcpStream::connect_timeout(&endpoint, Duration::from_secs(3)) {
                connected = Some(stream);
                break;
            }
        }
        let mut writer =
            connected.ok_or_else(|| format!("cannot connect to multiplayer server {address}"))?;
        writer
            .set_nodelay(true)
            .map_err(|error| format!("cannot configure multiplayer client: {error}"))?;
        writer
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|error| error.to_string())?;
        writer
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|error| error.to_string())?;
        // Finish the bounded handshake before spawning a reader. Any failed
        // handshake then closes every socket without leaving a detached reader.
        let mut reader = BufReader::new(
            writer
                .try_clone()
                .map_err(|error| format!("cannot clone multiplayer connection: {error}"))?,
        );
        send_message(
            &mut writer,
            &Message::Login {
                username: username.to_owned(),
                game_version: GAME_VERSION.to_owned(),
                protocol_version: PROTOCOL_VERSION,
            },
        )?;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let line = minicraft_common::protocol::read_bounded_line_until(&mut reader, Some(deadline))
            .map_err(|error| format!("multiplayer login failed: {error}"))?
            .ok_or("multiplayer server closed during login")?;
        let client_id = match decode_line(&line).map_err(|error| error.to_string())? {
            Message::Init {
                client_id,
                protocol_version: PROTOCOL_VERSION,
                server_version,
            } if client_id != 0 && server_version == GAME_VERSION => client_id,
            Message::Notify { text } => return Err(text),
            _ => return Err("expected compatible INIT as the first server message".to_owned()),
        };
        let (sender, receiver) = mpsc::sync_channel(256);
        let reader = thread::Builder::new()
            .name("multiplayer-reader".to_owned())
            .spawn(move || client_reader(reader, sender))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client_id,
            writer,
            receiver,
            reader: Some(reader),
        })
    }

    pub fn send_player(&mut self, x: i32, y: i32, level: usize, health: u8) -> Result<(), String> {
        send_message(
            &mut self.writer,
            &Message::Move {
                x,
                y,
                level,
                health,
            },
        )
    }

    pub fn ping(&mut self, nonce: u64) -> Result<(), String> {
        send_message(&mut self.writer, &Message::Ping { nonce })
    }

    pub fn poll(&mut self) -> Vec<Message> {
        self.receiver.try_iter().take(256).collect()
    }
}

impl Drop for MultiplayerClient {
    fn drop(&mut self) {
        let _ = send_message(&mut self.writer, &Message::Disconnect);
        let _ = self.writer.shutdown(Shutdown::Both);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn client_reader(mut reader: BufReader<TcpStream>, sender: SyncSender<Message>) {
    while let Ok(Some(line)) = minicraft_common::protocol::read_bounded_line_until(
        &mut reader,
        Some(std::time::Instant::now() + Duration::from_secs(35)),
    ) {
        let Ok(message) = decode_line(&line) else {
            break;
        };
        if matches!(
            message,
            Message::Login { .. }
                | Message::Move { .. }
                | Message::Ping { .. }
                | Message::Disconnect
                | Message::Init { .. }
        ) || sender.try_send(message).is_err()
        {
            break;
        }
    }
    let _ = reader.get_ref().shutdown(Shutdown::Both);
}

fn send_message(stream: &mut TcpStream, message: &Message) -> Result<(), String> {
    let frame = encode_line(message)
        .map_err(|error| format!("cannot encode multiplayer message: {error}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("cannot send multiplayer message: {error}"))
}
