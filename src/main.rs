use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn input() {
    let host = cpal::default_host();
    let devices = host.input_devices();

    if let Ok(devices) = devices {
        if let Some(device) = devices
            .into_iter()
            .filter(|x| x.name().unwrap().contains("pulse"))
            .next()
        {
            let config = device.default_input_config().unwrap();

            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        // Process audio here, `data` contains the PCM samples
                        println!("data: {:?}", data);
                    },
                    move |err| eprintln!("Stream error: {}", err),
                    None,
                )
                .unwrap();
            stream.play().unwrap();
        }
    }
}

fn main() {
    input();
}
