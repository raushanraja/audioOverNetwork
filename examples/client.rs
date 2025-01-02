use audiopus::SampleRate;
use audiopus::{coder::Decoder, Channels};
use rodio::buffer::SamplesBuffer;
use rodio::Sink;
use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:12346").expect("Failed to bind socket");
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    println!("Receiving audio...");

    let mut decoder = Decoder::new(SampleRate::Hz48000, Channels::Auto).unwrap();

    let mut buffer = [0u8; 4096]; // Buffer to receive audio packets
    let mut pcm_buffer = [0f32; 1920];
    loop {
        let (size, _src) = socket
            .recv_from(&mut buffer)
            .expect("Failed to receive data");

        let samples = decoder
            .decode_float(Some(&buffer[..size]), &mut pcm_buffer[..], false)
            .unwrap();

        let source = SamplesBuffer::new(2, 48000, &pcm_buffer[..samples]);
        sink.append(source);
    }
}
