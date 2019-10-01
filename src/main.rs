use crossbeam_channel::bounded;
use jack::{AudioIn, Client, ClientOptions};
use std::i16;

enum Message {
    Sample(Vec<( f32, f32 )>),
    Exit
}

fn main() {
    let client = Client::new("rust_capture", ClientOptions::NO_START_SERVER)
        .unwrap()
        .0;

    let in_spec = AudioIn::default();

    let audio_in_l_port = client.register_port("l", in_spec).unwrap();
    let audio_in_r_port = client.register_port("r", in_spec).unwrap();

    let sample_rate = client.sample_rate();

    let ( tx, rx ) = bounded::<Message>(5);

    let tx_1 = tx.clone();

    let buffer_size = client.buffer_size();

    println!("{}", buffer_size);

    let mut samples: Vec<(f32, f32)> = vec![(0., 0.); client.buffer_size() as usize];

    let system_out_ports = client.ports(None, None, jack::PortFlags::IS_OUTPUT);

    for s in system_out_ports.iter() {
        println!("Port: {}", s);
    }

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            let in_l = audio_in_l_port.as_slice(ps);
            let in_r = audio_in_r_port.as_slice(ps);

            for ( i, ( l, r )) in in_l.iter().zip(in_r.iter()).enumerate() {
                samples[i] = ( *l, *r );
            }
            
            tx.send(Message::Sample(samples.clone())).unwrap();

            jack::Control::Continue
        },
    );

    let active = client.activate_async((), process).unwrap();

    println!("Wire up rust_capture using a tool like Catia for Jack");
    println!("When you're ready, type a file name to start recording (.wav): ");

    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).ok();

    let user_input = user_input.trim();

    let wav_spec = hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(user_input, wav_spec).unwrap();

    let child = std::thread::spawn(move || {
        for message in rx.iter() {
            match message {
                Message::Sample(samples) => {
                    for ( l, r ) in samples.iter() {
                        writer.write_sample((l * (std::i16::MAX as f32)) as i16).unwrap();
                        writer.write_sample((r * (std::i16::MAX as f32)) as i16).unwrap();
                    }
                },
                Message::Exit => {
                    break;
                }
            }

        }

        writer.finalize().unwrap();
    });

    println!("Started recording {}", user_input);

    // Wait for user input to quit
    println!("Press enter to stop recording...");
    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).ok();

    tx_1.send(Message::Exit).unwrap();

    println!("Finishing up!");

    active.deactivate().unwrap();
    child.join().unwrap();

}
