use std::collections::btree_map::Keys;

use cpal::SampleRate;

fn encode_audio(data: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let sample_rate = 48000;
    let channels = opus::Channels::Stereo;
    let application = opus::Application::Audio;
    let bitrate = opus::Bitrate::Bits(24000);

    let mut encoder = opus::Encoder::new(sample_rate, channels, application)?;
    encoder.set_bitrate(bitrate)?;

    let frame_size = (sample_rate as i32 / 1000 * 200) as usize;

    let mut output = vec![0u8; 4096];
    let mut smaples_i = 0;
    let mut output_i = 0;
    let mut end_buffer = vec![0f32; frame_size];

    println!("Data length: {:?}", data.len());
    println!("Output length: {:?}", output.len());

    // // Store Number of samples
    {
        let samples: u32 = data.len().try_into()?;
        let bytes = samples.to_be_bytes();
        println!("Number of samples: {:?}, {:?}", samples, &bytes[..4]);
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

    Ok(output)
}

fn main() {
    let mut data = [0.0f32; 48000];

    data[0] = 1.0;
    data[1] = 1.0;
    data[2] = 1.0;
    data[3] = 1.0;

    let result = encode_audio(&data);

    println!("{:?}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_audio() {
        let mut data = [0.0f32; 48000];

        data[0] = 1.0;
        data[1] = 1.0;
        data[2] = 1.0;
        data[3] = 1.0;

        let result = encode_audio(&data);
        assert!(result.is_ok());
    }
}

