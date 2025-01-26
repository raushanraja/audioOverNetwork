use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::{io::AsyncWriteExt, net::TcpStream};

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

    let target_address = "192.168.0.163:12346"; // Replace with the receiver's address
                                                // let target_address = "0.0.0.0:12346"; // Replace with the receiver's address

    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    let sample_rate = config.sample_rate().0 as usize;
    let channels = config.channels() as usize;
    let buffer_size = sample_rate * channels * 2; // 2 seconds of audio
    let mut buffer = Vec::with_capacity(buffer_size);

    tokio::spawn(async move {
        while let Ok(data) = rx.recv() {
            buffer.extend(data);
            if buffer.len() >= buffer_size {
                match encode_audio(&buffer) {
                    Ok(encoded) => match TcpStream::connect(&target_address).await {
                        Ok(mut stream) => {
                            println!("Sending data {:?}, bytes:{:?}", buffer.len(), encoded.len());
                            if let Err(e) = stream.write_all(&encoded).await {
                                eprintln!("Failed to send data: {:?}", e);
                            }
                        }
                        Err(e) => eprintln!("Failed to connect to {}: {:?}", target_address, e),
                    },
                    Err(e) => eprintln!("Failed to encode audio data {:?}", e),
                }
                buffer.clear();
            }
        }
    });

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let data = data.to_vec();
                let tx = tx.clone();
                tokio::spawn(async move {
                    tx.send(data).unwrap();
                });
            },
            move |err| eprintln!("An error occurred on the input stream: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    println!("Streaming audio to {}", target_address);
    tokio::time::sleep(tokio::time::Duration::from_secs(1000)).await; // Stream for 5 seconds
}
