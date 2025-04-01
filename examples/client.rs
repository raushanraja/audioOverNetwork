use opus::{Channels, Decoder};
use rodio::buffer::SamplesBuffer;
use rodio::Sink;
use tokio::net::UdpSocket;

fn decode_audio(data: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let sample_rate = 48000;
    let channels = Channels::Stereo;
    let mut decoder = Decoder::new(sample_rate, channels)?;

    let mut output = Vec::new();
    let frame_size = (sample_rate as i32 / 1000 * 20) as usize;

    let mut input_i = 0;
    while input_i < data.len() {
        let packet = &data[input_i..];
        let mut decoded = vec![0f32; frame_size * channels as usize];
        match decoder.decode_float(packet, &mut decoded, false) {
            Ok(decoded_len) => {
                output.extend_from_slice(&decoded[..decoded_len * channels as usize]);
                input_i += packet.len();
            }
            Err(e) => {
                eprintln!("Failed to decode audio data: {:?}", e);
                return Err(Box::new(e));
            }
        }
    }

    Ok(output)
}

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("0.0.0.0:12346").await.expect("Failed to bind socket");
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    println!("Receiving audio...");

    let mut buffer = [0u8; 65536]; // Increase buffer size

    loop {
        let (size, _src) = socket.recv_from(&mut buffer).await.expect("Failed to receive data");

        let samples = match decode_audio(&buffer[..size]) {
            Ok(samples) => samples,
            Err(e) => {
                eprintln!("Failed to decode audio data {:?}", e);
                continue;
            }
        };

        let source = SamplesBuffer::new(2, 48000, samples);
        sink.append(source);
    }
}
