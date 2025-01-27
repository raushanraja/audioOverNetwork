use futures::AsyncWriteExt;

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut encoded = Vec::new();
    for sample in data {
        let sample = sample.to_le_bytes();
        encoded.extend_from_slice(&sample);
    }
    Ok(encoded)
}

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

fn main() {
    let mut data: Vec<f32> = vec![0.0; 10];
    for i in 0..data.len() {
        data[i] = i as f32;
    }

    let encoded = encode_audio(&data).unwrap();
    println!("Data: {:?}", data);
    println!("Encoded {} bytes", encoded.len());
    println!("Encoded data: {:?}", encoded);
    let binary = tungstenite::protocol::Message::Binary(encoded.clone().into());
    println!("{:?}", binary);

    let decoded = decode_audio(&encoded).unwrap();
    println!("Decoded {} samples", decoded.len());
}
