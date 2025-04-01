use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use tokio::net::UdpSocket;
use opus::{Application, Bitrate, Channels, Encoder};

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let sample_rate = 48000;
    let channels = Channels::Stereo;
    let application = Application::Audio;
    let bitrate = Bitrate::Bits(500000);

    let mut encoder = Encoder::new(sample_rate, channels, application)?;
    encoder.set_bitrate(bitrate)?;

    let frame_size = (sample_rate / 1000 * 20) as usize; // 20 ms frame size
    let frame_size = frame_size * 4 as usize;
    let mut output = Vec::new();
    let mut samples_i = 0;

    while samples_i < data.len() {
        let end = (samples_i + frame_size).min(data.len());
        let buff = &data[samples_i..end];
        let mut padded = vec![0f32; frame_size];

        if buff.len() < frame_size {
            padded[..buff.len()].copy_from_slice(buff);
        }

        match encoder.encode_vec_float(&padded, 48000) {
            Ok(result) => {
                println!("Encoded length: {:?}", result.len());
                output.extend_from_slice(&result);
                samples_i += frame_size;
            }
            Err(e) => {
                eprintln!("Failed to encode audio data: {:?}", e);
                return Err(Box::new(e));
            }
        }
    }

    Ok(output)
}

#[tokio::main]
async fn main() {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();

    println!(
        "Default input device: {}",
        device.name().unwrap_or("Unknown".to_string())
    );

    let socket = Arc::new(UdpSocket::bind("0.0.0.0:12345").await.expect("Failed to bind socket"));
    let target_address = "192.168.0.114:12346"; // Replace with the receiver's address

    let socket = Arc::clone(&socket);
    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let socket = Arc::clone(&socket);
                let data = data.to_vec(); // Copy data into a Vec
                tokio::spawn(async move {
                    match encode_audio(&data) {
                        Ok(result) => {
                            socket
                                .send_to(&result, target_address)
                                .await
                                .expect("Failed to send data");
                        }
                        Err(e) => {
                            eprintln!("Failed to encode audio data {:?}", e);
                        }
                    }
                });
            },
            move |err| eprintln!("Stream error: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();

    println!("Streaming audio to {}", target_address);
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; // Stream for 60 seconds
}
