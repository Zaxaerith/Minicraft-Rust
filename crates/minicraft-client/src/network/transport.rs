//! Line-delimited TCP transport for the shared multiplayer protocol.
use minicraft_common::protocol::{
    GAME_VERSION, Message, PROTOCOL_VERSION, decode_line, encode_line, read_bounded_line,
};
use std::{
    collections::VecDeque,
    io::{BufReader, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
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
    backlog: VecDeque<Message>,
}

impl MultiplayerClient {
    pub fn connect(address: &str, username: &str) -> Result<Self, String> {
        let mut writer = TcpStream::connect(address)
            .map_err(|error| format!("cannot connect to multiplayer server {address}: {error}"))?;
        writer
            .set_nodelay(true)
            .map_err(|error| format!("cannot configure multiplayer client: {error}"))?;
        let reader = writer
            .try_clone()
            .map_err(|error| format!("cannot clone multiplayer connection: {error}"))?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || client_reader(reader, sender));
        send_message(
            &mut writer,
            &Message::Login {
                username: username.to_owned(),
                game_version: GAME_VERSION.to_owned(),
                protocol_version: PROTOCOL_VERSION,
            },
        )?;
        let mut backlog = VecDeque::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let client_id = loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let message = receiver
                .recv_timeout(remaining)
                .map_err(|_| "multiplayer login timed out".to_owned())?;
            match message {
                Message::Init {
                    client_id,
                    protocol_version: PROTOCOL_VERSION,
                    ..
                } => break client_id,
                Message::Notify { text } => return Err(text),
                other => backlog.push_back(other),
            }
        };
        Ok(Self {
            client_id,
            writer,
            receiver,
            backlog,
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
        self.backlog
            .drain(..)
            .chain(self.receiver.try_iter())
            .collect()
    }
}

impl Drop for MultiplayerClient {
    fn drop(&mut self) {
        let _ = send_message(&mut self.writer, &Message::Disconnect);
    }
}

fn client_reader(stream: TcpStream, sender: Sender<Message>) {
    let mut reader = BufReader::new(stream);
    while let Ok(Some(line)) = read_bounded_line(&mut reader) {
        if let Ok(message) = decode_line(&line)
            && sender.send(message).is_err()
        {
            break;
        }
    }
}

fn send_message(stream: &mut TcpStream, message: &Message) -> Result<(), String> {
    let frame = encode_line(message)
        .map_err(|error| format!("cannot encode multiplayer message: {error}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("cannot send multiplayer message: {error}"))
}
