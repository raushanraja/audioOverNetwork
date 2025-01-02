use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::Duration;
use std::{net::SocketAddr, thread};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::{net::TcpListener, task, time::sleep};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};

fn input(tx: &mpsc::UnboundedSender<Vec<f32>>) {
    let host = cpal::default_host();
    let devices = host.input_devices();

    if let Ok(devices) = devices {
        if let Some(device) = devices
            .into_iter()
            .filter(|x| x.name().unwrap().contains("pulse"))
            .next()
        {
            println!("Default input device: {}", device.name().unwrap());
            let config = device.default_input_config().unwrap();
            let tsx = tx.clone();

            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Err(e) = tsx.send(data.to_vec()) {
                            eprintln!("Failed to send data: {:?}", e);
                        }
                    },
                    move |err| eprintln!("Stream error: {}", err),
                    None,
                )
                .unwrap();
            stream.play().unwrap();
            thread::sleep(Duration::from_secs(60 * 60 * 24));
            println!("Stream is playing");
        } else {
            println!("No suitable input device found");
        }
    } else {
        println!("Failed to get input devices");
    }
}

async fn handle_connection(
    raw_stream: tokio::net::TcpStream,
    mut rx: mpsc::UnboundedReceiver<Vec<f32>>,
) {
    let mut ws_stream = accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");

    tokio::spawn(async move {
        while let Some(sample) = rx.recv().await {
            let msg = Message::binary(String::from("Hello"));
            ws_stream
                .get_mut()
                .split()
                .1
                .write_all(&msg.into_data())
                .await
                .unwrap();
        }
    })
    .await
    .unwrap();
}

async fn process_request(req: &Request, res: Response) -> Result<Response, ErrorResponse> {
    println!("Processing request: {:?}", req);

    let headers = req.headers();
    let protocol = headers
        .get("Sec-WebSocket-Protocol")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("unknown");

    let username = headers
        .get("Sec-WebSocket-Username")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("unknown");

    let password = headers
        .get("Sec-WebSocket-Password")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("unknown");

    println!(
        "Protocol: {}, Username: {}, Password: {}",
        protocol, username, password
    );

    Ok(res)
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<f32>>();
    tokio::spawn(async move { input(&tx) });

    let server_connection_handler = tokio::spawn(async move {
        let listener = match TcpListener::bind("0.0.0.0:8181").await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to address: {}", e);
                return;
            }
        };

        while let Ok((stream, _)) = listener.accept().await {
            match tokio_tungstenite::accept_hdr_async(stream, process_request).await {
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
