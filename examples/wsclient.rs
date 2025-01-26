use anyhow::Result;
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::traits::{Producer,  Split, Consumer};
use rodio::DeviceTrait;
use std::net::UdpSocket;
use std::thread;

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
    let rb = ringbuf::SharedRb::new(1000000);
    let (mut producer, mut consumer) = rb.split();

    let socket = UdpSocket::bind("0.0.0.0:8080").expect("Couldn't bind to address");

    thread::spawn(move || {
        let mut buf = [0; 100000];
        loop {
            match socket.recv(&mut buf) {
                Ok(received) => {
                    if let Ok(samples) = decode_audio(&buf[..received]) {
                        for sample in samples {
                            if producer.try_push(sample).is_err() {
                                eprintln!("Failed to push audio data to ring buffer");
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Failed to receive audio data: {}", e),
            }
        }
    });

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    let config = device.default_output_config().unwrap();

    println!(
        "Default output device: {}, \n config: {:?}",
        device.name().unwrap_or("Unknown".to_string()),
        config
    );

    println!("Receiving audio...");

    let stream = device
        .build_output_stream(
            &config.clone().into(),
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for output_sample in output.iter_mut() {
                    if let Some(sample) = consumer.try_pop() {
                        *output_sample = sample;
                    } else {
                        *output_sample = 0.0; // Fill with silence if no data is available
                    }
                }
            },
            move |err| eprintln!("An error occurred on the output stream: {}", err),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1000)).await;
}
