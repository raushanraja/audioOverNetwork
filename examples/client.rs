use rodio::Sink;
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
        let audio_data = buffer[..size].to_vec();

        // Convert audio data to a format compatible with `rodio`
        let audio_cursor = Cursor::new(audio_data);
        let source = rodio::Decoder::new(audio_cursor).unwrap();

        sink.append(source);
    }
}
