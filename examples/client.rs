use cpal::traits::{HostTrait, StreamTrait};
use rodio::DeviceTrait;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

fn decode_audio(data: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    for chunk in data.chunks(4) {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(chunk);
        let sample = f32::from_le_bytes(bytes);
        output.push(sample);
    }
    Ok(output)
}

#[tokio::main]
async fn main() {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
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

    let listener = TcpListener::bind("0.0.0.0:12346")
        .await
        .expect("Failed to bind socket");

    println!("Receiving audio...");

    tokio::spawn(async move {
        loop {
            let (mut socket, _addr) = listener
                .accept()
                .await
                .expect("Failed to accept connection");
            let mut buffer = vec![0u8; 65536]; // Increase buffer size
            let size = socket.read(&mut buffer).await.expect("Failed to read data");
            match decode_audio(&buffer[..size]) {
                Ok(samples) => {
                    println!("Received {} samples", samples.len());
                    tx.send(samples).unwrap();
                }
                Err(e) => {
                    eprintln!("Failed to decode audio data {:?}", e);
                    continue;
                }
            };
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
