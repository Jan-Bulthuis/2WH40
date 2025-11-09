use std::f64::{self, consts::PI};

use crate::dist2;

#[derive(Debug)]
pub struct Room {
    pub sample_len: usize,
    pub signal_fs: usize,
    pub signal: Vec<f64>,
    pub v_sound: f64,
    pub room_arg: f64,
    pub room_amp: f64,
    pub source_arg: f64,
    pub source_amp: f64,
}

fn point_cost(room: &Room, distance: f64) -> f64 {
    let index = room.signal_fs as f64 * distance / room.v_sound;
    if index < room.signal.len() as f64 {
        room.signal[index as usize]
    } else {
        0.0
    }
}

fn project(pos: (f64, f64)) -> (f64, f64) {
    (pos.0 * pos.1.cos(), pos.0 * pos.1.sin())
}

pub fn virtual_sources(room: &Room, mic: &(f64, f64)) -> impl Iterator<Item = (f64, f64)> {
    let mic_arg = mic.1.atan2(mic.0);
    let source_arg = room.source_arg;
    let source_amp = room.source_amp;
    let room_arg = room.room_arg;

    let n_odd_min = ((mic_arg - PI - source_arg) / (2.0 * room_arg)).floor() as isize + 1;
    let n_odd_max = ((mic_arg + PI - source_arg) / (2.0 * room_arg)).ceil() as isize;
    let sources_odd =
        (n_odd_min..n_odd_max).map(move |n| (source_amp, source_arg + 2.0 * room_arg * n as f64));

    let n_even_min = ((mic_arg - PI + source_arg) / (2.0 * room_arg)).floor() as isize + 1;
    let n_even_max = ((mic_arg + PI + source_arg) / (2.0 * room_arg)).ceil() as isize;
    let sources_even = (n_even_min..n_even_max)
        .map(move |n| (source_amp, -source_arg + 2.0 * room_arg * n as f64));

    sources_odd.chain(sources_even)
}

pub fn sound_distances(room: &Room, mic: &(f64, f64)) -> Vec<f64> {
    let mut distances = virtual_sources(room, mic)
        .map(project)
        .map(|a| dist2(mic, &a))
        .collect::<Vec<f64>>();
    distances.sort_by(f64::total_cmp);
    distances
}

pub fn cost(room: &Room, mic: &(f64, f64), min_delta: f64) -> f64 {
    let distances = sound_distances(room, mic);
    let mut cost = 0.0;
    for i in 0..distances.len() {
        if i == 0 || (distances[i] - distances[i - 1]).abs() > min_delta {
            cost += point_cost(room, distances[i]);
        }
    }
    cost
}

pub fn localize(room: &Room) -> (f64, f64) {
    let sample_distance: f64 = room.v_sound * room.sample_len as f64 / (room.signal_fs as f64);
    let min_step: f64 = sample_distance * 0.1;
    let min_delta: f64 = sample_distance * 0.5 / 2.0f64.sqrt();

    let xi_min = (-room.room_amp / min_step).floor() as isize;
    let xi_max = (room.room_amp / min_step).ceil() as isize;
    (xi_min..xi_max)
        .flat_map(|xi| {
            let x = xi as f64 * min_step;
            let yi_min = if room.room_arg <= f64::consts::FRAC_PI_2 {
                1
            } else {
                1.max((xi as f64 * room.room_arg.tan()).floor() as isize + 1)
            };
            let yi_max = (((10.0f64).powi(2) - x.powi(2)).sqrt() / min_step) as isize;
            let yi_max = if room.room_arg < f64::consts::FRAC_PI_2 {
                yi_max.min((xi as f64 * room.room_arg.tan()).ceil() as isize - 1)
            } else {
                yi_max
            };

            (yi_min..yi_max).map(move |yi| {
                let y = yi as f64 * min_step;
                (x, y)
            })
        })
        .reduce(|a, b| {
            if cost(&room, &a, min_delta) >= cost(&room, &b, min_delta) {
                a
            } else {
                b
            }
        })
        .unwrap_or((-20.0, -20.0))
}
