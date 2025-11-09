use std::{fmt::Debug, fs::File, path::PathBuf, process::Output, sync::Mutex};

use hound::WavReader;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

mod angled;
mod parallel;

#[derive(Serialize, Deserialize, Debug)]
struct Room {
    id: String,
    source: Vec<f64>,
    mics: Vec<Vec<f64>>,
    sample: String,
    simulated_audio: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LocalizeResult {
    label: String,
    angled: bool,
    wall: f64,
    source: (f64, f64),
    mic: (f64, f64),
    guess: (f64, f64),
    mic_distances: Vec<f64>,
    guess_distances: Vec<f64>,
}

fn main() {
    let data_path = PathBuf::from("generations.json");
    let data_file = File::open(data_path).unwrap();
    let result_path = PathBuf::from("localization.json");
    let result_file = File::create(result_path).unwrap();
    let data: Vec<Room> = serde_json::from_reader(data_file).unwrap();
    let results = data
        .par_iter()
        .map(localize_room)
        .collect::<Vec<LocalizeResult>>();
    serde_json::to_writer_pretty(result_file, &results).unwrap();
}

// Attempt to perform localization inside a room
fn localize_room(room: &Room) -> LocalizeResult {
    let sample: Vec<f64> = WavReader::open(&format!("Samples/{}.wav", room.sample))
        .unwrap()
        .samples::<i16>()
        .map(|sample| sample.expect("Sample must have 16bit samples").into())
        .collect();
    for (mic, audio_file) in room.mics.iter().zip(room.simulated_audio.iter()) {
        let mic = (mic[0], mic[1]);
        let signal: Vec<f64> = WavReader::open(PathBuf::from("Simulated").join(audio_file))
            .unwrap()
            .samples::<f32>()
            .map(|sample| {
                sample
                    .expect("Simulated noise must have f32 samples")
                    .into()
            })
            .collect();

        let signal = process_signal(&signal, &sample);

        if room.id.starts_with("2wall_parallel") {
            let parts: Vec<&str> = room.id.split("_").collect();
            let room_width = parts[2].parse().unwrap();
            let source_x = parts[3].parse().unwrap();

            let room_par = parallel::Room {
                sample_len: sample.len(),
                signal_fs: 48000,
                signal: signal,
                v_sound: 343.0,
                width: room_width,
                source_x: source_x,
            };
            let result = parallel::localize(&room_par);
            let result_distances = parallel::sound_distances(&room_par, &result, 3);
            let actual_distances = parallel::sound_distances(&room_par, &mic, 3);
            println!();
            println!("Parallel room {}", room.id);
            println!("Guess: {result:?}");
            println!("Actual: {mic:?}");
            println!("Difference: {}", dist2(&result, &mic));
            println!("Guess distances: {result_distances:?}");
            println!("Actual distances: {actual_distances:?}");
            return LocalizeResult {
                label: room.id.clone(),
                angled: false,
                wall: room_par.width,
                source: (room_par.source_x, 0.0),
                mic: mic,
                guess: result,
                mic_distances: actual_distances,
                guess_distances: result_distances,
            };
        } else if room.id.starts_with("2wall_angled") {
            let parts: Vec<&str> = room.id.split("_").collect();
            let room_arg = parts[2].parse().unwrap();
            let source_arg = parts[3].parse().unwrap();
            let source_amp = parts[4].parse().unwrap();

            let arg_room = angled::Room {
                sample_len: sample.len(),
                signal_fs: 48000,
                signal: signal,
                v_sound: 343.0,
                room_arg: room_arg,
                room_amp: 10.0,
                source_arg: source_arg,
                source_amp: source_amp,
            };
            let result = angled::localize(&arg_room);
            let result_distances = angled::sound_distances(&arg_room, &result);
            let actual_distances = angled::sound_distances(&arg_room, &mic);
            let guess_score = angled::cost(&arg_room, &result, 0.1429);
            let actual_score = angled::cost(&arg_room, &mic, 0.1429);
            println!();
            println!("Angled room {}", room.id);
            println!("Guess: {result:?}");
            println!("Actual: {mic:?}");
            println!("Difference: {}", dist2(&result, &mic));
            println!("Guess distances: {result_distances:?}");
            println!("Actual distances: {actual_distances:?}");
            println!("Guess score: {guess_score}");
            println!("Actual score: {actual_score}");
            return LocalizeResult {
                label: room.id.clone(),
                angled: true,
                wall: arg_room.room_arg,
                source: (
                    arg_room.source_amp * arg_room.source_arg.cos(),
                    arg_room.source_amp * arg_room.source_arg.sin(),
                ),
                mic: mic,
                guess: result,
                mic_distances: actual_distances,
                guess_distances: result_distances,
            };
        } else {
            unimplemented!()
        }
    }
    panic!()
}

fn process_signal(signal: &Vec<f64>, sample: &Vec<f64>) -> Vec<f64> {
    let signal: Vec<f64> = signal
        .windows(sample.len())
        .map(|slice| slice.iter().zip(sample.iter()).map(|(a, b)| a * b).sum())
        .collect();
    let max = signal.iter().map(|s| *s).reduce(f64::max).unwrap();
    signal.iter().map(|s| s / max).collect()
}

fn dist2(a: &(f64, f64), b: &(f64, f64)) -> f64 {
    return ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
}
