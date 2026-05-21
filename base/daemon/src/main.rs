use protocol::OpusHandler;

fn main() {
    let mut oh = OpusHandler::new();

    let mut reader = hound::WavReader::open("./test2.wav").unwrap();
    println!("{}", reader.spec().sample_rate);
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();
    let mut i = 0;
    while samples.len() > i + OpusHandler::AUDIO_BUFFER_SIZE {
        let mut input = [0i16; OpusHandler::AUDIO_BUFFER_SIZE];
        input.clone_from_slice(&samples.as_slice()[i..i + OpusHandler::AUDIO_BUFFER_SIZE]);
        i += OpusHandler::AUDIO_BUFFER_SIZE;

        let mut middle = OpusHandler::make_opus_buffer();
        let mut output = OpusHandler::make_audio_buffer();

        let size = oh.encode(&input, &mut middle).unwrap();
        oh.decode(&middle, size, &mut output);

        for sample in output {
            let _ = writer.write_sample(sample);
        }

        //for (inp, out) in output.iter().zip(input) {
        //    println!("{inp} | {out} | {}", inp - out);
        //}
    }

    let _ = writer.finalize();
}
