use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::UdpSocket;

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let sample_rate = 48000;
    let channels = opus::Channels::Stereo;
    let application = opus::Application::LowDelay;
    let bitrate = opus::Bitrate::Bits(48000);

    let mut encoder = opus::Encoder::new(sample_rate, channels, application)?;
    encoder.set_bitrate(bitrate)?;

    let frame_rate_ms = 20;
    let frame_rate = 1000 / frame_rate_ms;
    let frame_size = (sample_rate as i32 / frame_rate) as usize;

    let mut output = vec![0u8; 48000];
    let mut samples_i = 0;
    let mut output_i = 0;
    let mut end_buffer = vec![0f32; frame_size];

    while samples_i < data.len() {
        let buff = if samples_i + frame_size < data.len() {
            &data[samples_i..samples_i + frame_size]
        } else {
            let end = data.len() - samples_i;
            end_buffer[..end].copy_from_slice(&data[samples_i..]);
            &end_buffer
        };

        match encoder.encode_vec_float(&buff, 4096) {
            Ok(result) => {
                if output_i + result.len() > output.len() {
                    output.resize(output_i + result.len(), 0);
                }
                output[output_i..output_i + result.len()].copy_from_slice(&result);
                output_i += result.len();
                samples_i += frame_size;
            }
            Err(e) => {
                eprintln!("Failed to encode audio data {:?}", e);
                match e.code() {
                    opus::ErrorCode::BufferTooSmall => {
                        eprintln!("Output buffer is too small for the copy operation");
                    }
                    _ => return Err(Box::new(e)),
                }
            }
        }
    }

    // Trim the output buffer to the actual size
    output.truncate(output_i);
    Ok(output)
}
fn main() {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();

    println!(
        "Default input device: {}",
        device.name().unwrap_or("Unknown".to_string())
    );

    let socket = UdpSocket::bind("0.0.0.0:12345").expect("Failed to bind socket");
    let target_address = "192.168.0.114:12346"; // Replace with the receiver's address
                                                // let target_address = "0.0.0.0:12346"; // Replace with the receiver's address

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| match encode_audio(data) {
                Ok(result) => {
                    socket
                        .send_to(&result, target_address)
                        .expect("Failed to send data");
                }
                Err(e) => {
                    eprintln!("Failed to encode audio data {:?}", e);
                }
            },
            move |err| eprintln!("Stream error: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();

    println!("Streaming audio to {}", target_address);
    std::thread::sleep(std::time::Duration::from_secs(60)); // Stream for 60 seconds
}
