use anyhow::Result;
use cpal::traits::{HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use rodio::DeviceTrait;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::connect_async;

pub struct WSMessage {
    pub message: Vec<u8>,
}

pub struct WebSocketClient {
    url: String,
    tx: Sender<WSMessage>,
    rx: Arc<Mutex<Receiver<WSMessage>>>,
}

pub trait Client {
    fn connect(&self) -> impl std::future::Future<Output = Result<()>> + Send;
    fn reconnect(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl WebSocketClient {
    pub fn new(url: String, tx: Sender<WSMessage>, rx: Arc<Mutex<Receiver<WSMessage>>>) -> Self {
        WebSocketClient { url, tx, rx }
    }

    async fn try_connect(&self) -> Result<(), anyhow::Error> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut sink, mut stream) = ws_stream.split();
        let rx = self.rx.clone();
        tokio::spawn(async move {
            let mut rx = rx.lock().await;
            while let Some(msg) = rx.recv().await {
                println!("Sending message: {:?}", msg.message);
                sink.send(tokio_tungstenite::tungstenite::Message::Binary(
                    msg.message.into(),
                ))
                .await
                .expect("Failed to send message");
            }
        });

        let tx = self.tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                let msg = msg.expect("Failed to receive message");
                let msg = tungstenite::Message::Binary(msg.into_data());
                let msg: Vec<u8> = msg.into_data().to_vec();
                crate::decode_audio(&msg).expect("Failed to decode audio data");
                tx.send(WSMessage { message: msg })
                    .await
                    .expect("Failed to send message");
            }
        });

        println!("WebSocket handshake has been successfully completed");
        Ok(())
    }

    async fn connect(&self) -> Result<(), anyhow::Error> {
        match self.try_connect().await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Failed to connect to WebSocket: {:?}", e);
                self.reconnect().await
            }
        }
    }

    async fn reconnect(&self) -> anyhow::Result<()> {
        let mut attempt = 0;
        let max_attempts = 5;
        let mut delay = Duration::from_secs(1);

        while attempt < max_attempts {
            if let Err(e) = self.try_connect().await {
                eprintln!(
                    "Failed to reconnect: {:?}. Attempt {}/{}",
                    e,
                    attempt + 1,
                    max_attempts
                );
                attempt += 1;
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            } else {
                println!(
                    "Reconnected successfully on attempt {}/{}",
                    attempt + 1,
                    max_attempts
                );
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Failed to reconnect after {} attempts",
            max_attempts
        ))
    }
}

impl Client for WebSocketClient {
    async fn connect(&self) -> Result<(), anyhow::Error> {
        self.connect().await
    }

    async fn reconnect(&self) -> anyhow::Result<()> {
        self.reconnect().await
    }
}

fn decode_audio(data: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    let mut bytes = [0u8; 4];
    for i in 0..(data.len() / 4) {
        bytes.copy_from_slice(&data[i * 4..(i + 1) * 4]);
        let sample = f32::from_le_bytes(bytes);
        output.push(sample);
    }
    Ok(output)
}

#[tokio::main]
async fn main() {
    let (tx_in, mut rx_in) = tokio::sync::mpsc::channel::<WSMessage>(100);
    let (_tx_out, rx_out) = tokio::sync::mpsc::channel::<WSMessage>(100);
    let url = "ws://localhost:8080";
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();

    tokio::spawn(async move {
        let ws_client = WebSocketClient::new(url.to_string(), tx_in, Arc::new(Mutex::new(rx_out)));
        ws_client.connect().await.unwrap();
    });

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_output_config().unwrap();

    println!(
        "Default output device: {}, \n config: {:?}",
        device.name().unwrap_or("Unknown".to_string()),
        config
    );

    println!("Receiving audio...");

    tokio::spawn(async move {
        while let Some(msg) = rx_in.recv().await {
            println!("Received message: {:?}", msg.message);
            if let Ok(samples) = decode_audio(&msg.message) {
                println!("Received {} samples", samples.len());
                tx.send(samples).unwrap();
            }
        }
    });

    let stream = device
        .build_output_stream(
            &config.clone().into(),
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let data = match rx.try_recv() {
                    Ok(data) => data,
                    Err(_) => return,
                };
                println!("Playing {} samples", data.len());
                for (output_sample, input_sample) in output.iter_mut().zip(data.iter()) {
                    *output_sample = *input_sample;
                }
            },
            move |err| eprintln!("An error occurred on the output stream: {}", err),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1000)).await;
}
