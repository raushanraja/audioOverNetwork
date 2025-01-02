use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::UdpSocket;

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let sample_rate = 48000;
    let channels = opus::Channels::Stereo;
    let application = opus::Application::Audio;
    let bitrate = opus::Bitrate::Bits(24000);

    let mut encoder = opus::Encoder::new(sample_rate, channels, application)?;
    encoder.set_bitrate(bitrate)?;

    let frame_size = (sample_rate as i32 / 1000 * 20) as usize;

    let mut output = vec![0u8; 4096];
    let mut smaples_i = 0;
    let mut output_i = 0;
    let mut end_buffer = vec![0f32; frame_size];

    // // Store Number of samples
    {
        let samples: u32 = data.len().try_into()?;
        let bytes = samples.to_be_bytes();
        // println!("Number of samples: {:?}, {:?}", samples, &bytes[..4]);
        if output.len() >= output_i + 4 {
            output[output_i..output_i + 4].copy_from_slice(&bytes);
        } else {
            // Handle the error, e.g., log a message or resize the output buffer
            eprintln!("Output buffer is too small for the copy operation");
        }
        output_i += 4;
    }
    while smaples_i < data.len() {
        let buff = if smaples_i + frame_size < data.len() {
            &data[smaples_i..smaples_i + frame_size]
        } else {
            let end = data.len() - smaples_i;
            end_buffer[..end].copy_from_slice(&data[smaples_i..]);
            &end_buffer
        };

        match encoder.encode_vec_float(&buff, 4096) {
            Ok(result) => {
                output[output_i..output_i + result.len()].copy_from_slice(&result);
                output_i += result.len();
                smaples_i += frame_size;
            }
            Err(e) => {
                eprintln!("Failed to encode audio data {:?}", e);
                match e.code() {
                    opus::ErrorCode::BufferTooSmall => {}
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

    let socket = UdpSocket::bind("0.0.0.0:12345").expect("Failed to bind socket");
    // let target_address = "192.168.0.114:12346"; // Replace with the receiver's address
    let target_address = "0.0.0.0:12346"; // Replace with the receiver's address

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
