use aprocess::ws;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc;

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut encoded = Vec::new();
    for sample in data {
        let sample = sample.to_le_bytes();
        encoded.extend_from_slice(&sample);
    }
    Ok(encoded)
}

#[tokio::main]
async fn main() {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();

    println!(
        "Default input device: {} \n config: {:?}",
        device.name().unwrap_or("Unknown".to_string()),
        config
    );

    let (tx, audio_rx) = mpsc::channel::<Vec<u8>>(100000);
    let (txin, audio_rxin) = std::sync::mpsc::channel::<Vec<f32>>();
    tokio::spawn(async move {
        ws::run(audio_rx).await;
    });

    tokio::spawn(async move {
        loop {
            match audio_rxin.recv() {
                Ok(data) => {
                    // println!("Received {} samples", data.len());
                    let data = encode_audio(&data).unwrap();
                    if let Err(e) = tx.send(data).await {
                        eprintln!("Failed to receive audio data: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to receive audio data: {}", e);
                }
            }
        }
    });

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let data = data.to_vec();
                let txin = txin.clone();
                std::thread::spawn(move || {
                    if let Err(e) = txin.send(data) {
                        eprintln!("Failed to send audio data: {}", e);
                    }
                });
            },
            move |err| eprintln!("An error occurred on the input stream: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    println!("Streaming audio to");
    tokio::time::sleep(tokio::time::Duration::from_secs(1000)).await; // Stream for 5 seconds
}
