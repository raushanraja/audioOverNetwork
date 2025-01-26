use futures_util::*;
use rand;
use rodio::buffer;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tokio_tungstenite::WebSocketStream;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::Message;

// Define a struct to hold client's connection state
#[derive(Debug, Clone)]
struct CClient {
    id: u64,
    sender: mpsc::UnboundedSender<Vec<u8>>,
}

// Define a enum for server messages sebt bwtween the
// server and client threads
#[derive(Debug, Clone)]
enum ServerMessage {
    NewClient(CClient),
    Message(u64, String),
    Broadcast(Vec<u8>),
    RemoveClient(u64),
}

// Define a struct to hold the server state
struct SServer {
    clients: Arc<RwLock<HashMap<u64, CClient>>>,
    tx: mpsc::UnboundedSender<ServerMessage>,
}

// MEthod to get a randomId
fn next_id() -> u64 {
    rand::random()
}

// Define a method to handle incoming messages
fn handle_message(server: Arc<SServer>, msg: ServerMessage) {
    match msg {
        ServerMessage::NewClient(client) => {
            println!("New client connected: {}", client.id);
            match server.clients.try_write() {
                Ok(mut clients) => {
                    clients.insert(client.id, client);
                }
                Err(e) => {
                    eprintln!("Failed to acquire write lock: {}", e);
                }
            }
        }
        ServerMessage::Message(id, message) => match server.clients.try_read() {
            Ok(clients) => {
                if let Some(client) = clients.get(&id) {
                    println!(
                        "Sending new message client_id: {}, message: {}",
                        client.id, message
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to acquire read lock: {}", e);
            }
        },
        ServerMessage::RemoveClient(id) => match server.clients.try_write() {
            Ok(mut clients) => {
                clients.remove(&id);
                println!("client {} disconnected", id);
            }
            Err(e) => {
                eprintln!("Failed to acquire write lock: {}", e);
            }
        },
        ServerMessage::Broadcast(message) => match server.clients.try_read() {
            Ok(clients) => {
                for (_, client) in clients.iter() {
                    if let Err(e) = client.sender.send(message.clone()) {
                        eprintln!("Failed to send message: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to acquire read lock: {}", e);
            }
        },
    }
}
// Define a method to handle incoming WebSocket connections
async fn handle_connection(
    tx: mpsc::UnboundedSender<ServerMessage>,
    ws_stream: WebSocketStream<TcpStream>,
) {
    let _count = 0;
    let (mut outgoing, incoming) = ws_stream.split();
    let client_id = next_id();
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    let client = CClient {
        id: client_id,
        sender: message_tx.clone(),
    };

    if let Err(e) = tx.send(ServerMessage::NewClient(client.clone())) {
        eprintln!("Failed to send new client message: {}", e);
    }
    let txc: mpsc::UnboundedSender<ServerMessage> = tx.clone();

    // let mut ping_interval = interval(Duration::from_secs(1));
    let mut incoming = incoming.map_err(|e| {
        println!("Error {}", e);
        // txc.send(ServerMessage::RemoveClient(client_id)).unwrap();
    });

    let _msc = message_tx.clone();
    loop {
        tokio::select! {
                    message = message_rx.recv() =>{
                        match  message {
                                Some(msg) => {
                                    let _ = outgoing.send(Message::Binary(msg.clone().into())).await;
                                },
                                None => println!("None"),
                            }
                    },

        msg = incoming.next() => {
            match msg {
                Some(Ok(msg)) => {
                    // handle_message(server, msg)
                    println!("Received message: {}", msg);
                    if let Err(e) = txc.send(ServerMessage::Message(client_id, msg.to_string())) {
                        eprintln!("Failed to send message: {}", e);
                    }
                }
                _ => {
                    if let Err(e) = txc.send(ServerMessage::RemoveClient(client_id)) {
                        eprintln!("Failed to send remove client message: {}", e);
                    }
                    break;
                }
            }
        }
                    // _ = ping_interval.tick() => {
                    //        // txc.send(ServerMessage::Message(client_id, String::from("Ping"))).unwrap();
                    // }
                }
    }
}

fn process_header(req: &Request, res: Response) -> Result<Response, ErrorResponse> {
    Ok(res)
}

pub async fn run(mut audio_rx: std::sync::mpsc::Receiver<Vec<u8>>) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let txc = tx.clone();
    let server = Arc::new(SServer {
        clients: Arc::new(RwLock::new(HashMap::new())),
        tx,
    });
    let message_handler_server = server.clone();

    std::thread::spawn(move || {
        loop {
            match audio_rx.recv() {
                Ok(data) => {
                    // println!("Received {} samples", data.len());
                    let data = data;
                    // println!("Encoded {} bytes : 22", data.len());
                    if let Err(e) = txc.send(ServerMessage::Broadcast(data)) {
                        eprintln!("Failed to receive audio data: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to receive audio data: {}", e);
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            handle_message(message_handler_server.clone(), message);
        }
    });

    tokio::spawn(async move {
        let listener = match TcpListener::bind("0.0.0.0:8080").await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to address: {}", e);
                return;
            }
        };

        while let Ok((stream, _)) = listener.accept().await {
            println!("Accepted connection");
            let tx = server.tx.clone();
            match tokio_tungstenite::accept_hdr_async(stream, process_header).await {
                Ok(connection) => {
                    let join_handle = handle_connection(tx, connection);
                    tokio::spawn(join_handle);
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    });
}
