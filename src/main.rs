use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::UdpSocket;

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

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
    socket.connect("127.0.0.1:8080").expect("Couldn't connect to server");

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let data = data.to_vec();
                let socket = socket.try_clone().expect("Failed to clone socket");
                std::thread::spawn(move || {
                    let encoded_data = encode_audio(&data).unwrap();
                    if let Err(e) = socket.send(&encoded_data) {
                        eprintln!("Failed to send audio data: {}", e);
                    }
                });
            },
            move |err| eprintln!("An error occurred on the input stream: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    println!("Streaming audio to UDP server");
    tokio::time::sleep(tokio::time::Duration::from_secs(1000)).await; // Stream for 1000 seconds
}
