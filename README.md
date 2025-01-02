# audioOverNetwork - Audio Streaming Example
This project demonstrates a simple audio streaming setup using Rust. It includes a server that captures audio from an input device and sends it over UDP, and a client that receives the audio data and plays it back.

## Dependencies

- [cpal](https://crates.io/crates/cpal): Cross-platform audio I/O library.
- [rodio](https://crates.io/crates/rodio): Rust audio playback library.
- [tokio](https://crates.io/crates/tokio): Asynchronous runtime for Rust.
- [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite): WebSocket library for Tokio.
- [tungstenite](https://crates.io/crates/tungstenite): WebSocket library.

## Setup

1. Add the dependencies to your `Cargo.toml` file:

    ```toml
    [dependencies]
    bytemuck = "1.21.0"
    cpal = "0.15.3"
    rodio = "0.20.1"
    tokio = { version = "1.42.0", features = ["full"] }
    tokio-tungstenite = "0.26.1"
    tungstenite = "0.26.1"
    ```

2. Build the project:

    ```sh
    <!-- cargo build --> The Code is present in examples directory
    ```

## Running the Server

The server captures audio from the default input device and sends it over UDP to the specified address.

```sh
cargo run --example server
```

## Running the Client

The client receives audio data over UDP and plays it back using `rodio`.

```sh
cargo run --example client
```

## Example Code

### Server

```rust
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
```

### Client

```rust
use rodio::{Sink};
use std::io::Cursor;
use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:12346").expect("Failed to bind socket");
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    println!("Receiving audio...");

    let mut buffer = [0u8; 4096]; // Buffer to receive audio packets
    loop {
        let (size, _src) = socket
            .recv_from(&mut buffer)
            .expect("Failed to receive data");
        let audio_data: &[u8] = &buffer[..size]; // Use byte slice directly

        // Convert audio data to a format compatible with `rodio`
        let audio_cursor = Cursor::new(audio_data);
        let source = rodio::Decoder::new(audio_cursor).unwrap();
        
        sink.append(source);
    }
}
```

## License

This project is licensed under the MIT License.
