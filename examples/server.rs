use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::UdpSocket;

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:12345").expect("Failed to bind socket");
    let target_address = "127.0.0.1:12346"; // Replace with the receiver's address

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Serialize and send captured audio data over UDP
                let bytes = bytemuck::cast_slice(data); // Convert f32 slice to byte slice
                socket
                    .send_to(bytes, target_address)
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
