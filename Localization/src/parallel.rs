use crate::dist2;

#[derive(Debug)]
pub struct Room {
    pub sample_len: usize,
    pub signal_fs: usize,
    pub signal: Vec<f64>,
    pub v_sound: f64,
    pub width: f64,
    pub source_x: f64,
}

fn point_cost(room: &Room, distance: f64) -> f64 {
    let index = room.signal_fs as f64 * distance / room.v_sound;
    room.signal[index as usize]
}

fn virtual_sources(
    room: &Room,
    _mic: &(f64, f64),
    max_order: usize,
) -> impl Iterator<Item = (f64, f64)> {
    let n_odd: isize = (max_order as isize + 1) / 2;
    let sources_odd =
        ((-n_odd + 1)..(n_odd + 1)).map(|n| (-room.source_x + 2.0 * room.width * n as f64, 0.0));

    let n_even: isize = (max_order as isize / 2) + 1;
    let sources_even =
        ((-n_even + 1)..n_even).map(|n| (room.source_x + 2.0 * room.width * n as f64, 0.0));

    sources_odd.chain(sources_even)
}

pub fn sound_distances(room: &Room, mic: &(f64, f64), max_order: usize) -> Vec<f64> {
    let mut distances = virtual_sources(room, mic, max_order)
        .map(|a| dist2(mic, &a))
        .collect::<Vec<f64>>();
    distances.sort_by(f64::total_cmp);
    distances
}

fn cost(room: &Room, mic: &(f64, f64), max_order: usize, min_delta: f64) -> f64 {
    let distances = sound_distances(room, mic, max_order);
    let mut cost = 0.0;
    for i in 0..distances.len() {
        if i == 0 || (distances[i] - distances[i - 1]).abs() > min_delta {
            cost += point_cost(room, distances[i]);
        }
    }
    cost
}

pub fn localize(room: &Room) -> (f64, f64) {
    let max_order = 3;
    let sample_distance: f64 = room.v_sound * room.sample_len as f64 / (room.signal_fs as f64);
    let min_step: f64 = sample_distance * 0.1;
    let min_delta: f64 = sample_distance * 0.5;

    let yi_min = 0;
    let yi_max = (10.0 / min_step).ceil() as usize;
    (yi_min..yi_max)
        .flat_map(|yi| {
            let y = yi as f64 * min_step;
            let xi_min = 1;
            let xi_max = (room.width / min_step).ceil() as usize;
            (xi_min..xi_max).map(move |xi| {
                let x = xi as f64 * min_step;
                (x, y)
            })
        })
        .reduce(|a, b| {
            if cost(&room, &a, max_order, min_delta) >= cost(&room, &b, max_order, min_delta) {
                a
            } else {
                b
            }
        })
        .unwrap_or((-20.0, -20.0))
}
