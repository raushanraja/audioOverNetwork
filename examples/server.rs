use audiopus::{coder::Encoder, Application, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::UdpSocket;

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:12345").expect("Failed to bind socket");
    let target_address = "192.168.0.114:12346"; // Replace with the receiver's address

    // Setup Opus encoder
    let encoder = Encoder::new(
        SampleRate::Hz48000,
        audiopus::Channels::Auto,
        Application::Audio,
    )
    .expect("Failed to create encoder");

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Serialize and send captured audio data over UDP
                //
                let mut output = [0u8; 4096];
                let len = encoder.encode_float(data, &mut output).unwrap();
                socket
                    .send_to(&output[..len], target_address)
                    .expect("Failed to send data");
            },
            move |err| eprintln!("Stream error: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();

    println!("Streaming audio to {}", target_address);
    std::thread::sleep(std::time::Duration::from_secs(60)); // Stream for 60 seconds
}
