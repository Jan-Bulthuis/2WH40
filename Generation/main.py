import random
import numpy as np
import polars as pl
import pyroomacoustics as pra
from scipy.io import wavfile


# Helper function to generate rooms consisting of two parallel walls
def generate_parallel_room(distance, source_x, mic_x, mic_y):
    return {
        "room_id": f"2wall_parallel_{distance}_{source_x}",
        "type": "2D",
        "source": [source_x, 0.0],
        "mics": [(mic_x, mic_y)],
        "polygon": [[0.0, 10.0], [0.0, -10.0], [distance, -10.0], [distance, 10.0]],
        "materials": ["hard_surface", "anechoic", "hard_surface", "anechoic"],
    }


# Helper function to generate rooms consisting of two angled walls
def generate_angle_room(angle, source_angle, source_amp, mic_angle, mic_amp):
    angle = angle * np.pi
    source_angle = source_angle * np.pi
    mic_angle = mic_angle * np.pi
    return {
        "room_id": f"2wall_angled_{angle}_{source_angle}_{source_amp}",
        "type": "2D",
        "source": [
            source_amp * np.cos(source_angle),
            source_amp * np.sin(source_angle),
        ],
        "mics": [(mic_amp * np.cos(mic_angle), mic_amp * np.sin(mic_angle))],
        "polygon": [
            [0.0, 0.0],
            [10.0, 0.0],
            [20.0 * np.cos(angle / 2.0), 20.0 * np.sin(angle / 2.0)],
            [10.0 * np.cos(angle), 10.0 * np.sin(angle)],
        ],
        "materials": ["hard_surface", "anechoic", "anechoic", "hard_surface"],
    }


# Generate a room with a microphone placed uniformly at random inside it
def generate_fixed_uniform_angle_room(room_arg, source_arg, source_amp):
    mic_arg = random.uniform(0.0, room_arg)
    mic_amp = np.sqrt(random.uniform(0.0, 36))
    return generate_angle_room(room_arg, source_arg, source_amp, mic_arg, mic_amp)


# Genarate k identical rooms with the microphone at varying locations
def generate_fixed_uniform_angle_rooms(room_arg, source_arg, source_amp, k):
    return pl.DataFrame(
        [
            generate_fixed_uniform_angle_room(room_arg, source_arg, source_amp)
            for _ in range(k)
        ]
    )


# Generate an angled room configuration uniformly at random
def generate_uniform_angle_room():
    room_arg = random.uniform(0.0, 2.0 / 3.0)
    source_arg = random.uniform(0.0, room_arg)
    if room_arg > 0.33 and room_arg < 0.5:
        if random.random() <= 0.5:
            source_arg = random.uniform(0.0, 1 - 2 * room_arg)
        else:
            source_arg = random.uniform(3 * room_arg - 1, room_arg)
    elif room_arg >= 0.5:
        source_arg = random.uniform(2 * room_arg - 1, 1 - room_arg)
    mic_arg = random.uniform(0.0, room_arg)
    source_amp = np.sqrt(random.uniform(0.0, 100.0))
    mic_amp = np.sqrt(random.uniform(0.0, 100.0))
    return generate_angle_room(room_arg, source_arg, source_amp, mic_arg, mic_amp)


# Generate k angled room configurations sampled uniformly at random
def generate_uniform_angle_rooms(k):
    return pl.DataFrame([generate_uniform_angle_room() for _ in range(k)])


# Generate a parallel room with a microphone placed uniformly at random inside it
def generate_fixed_uniform_parallel_room(width, source_x):
    mic_x = random.uniform(0.0, width)
    mic_y = random.uniform(0.0, 10.0)
    return generate_parallel_room(width, source_x, mic_x, mic_y)


# Generate k identical rooms with the microphone at varying locations
def generate_fixed_uniform_parallel_rooms(width, source_x, k):
    return pl.DataFrame(
        [generate_fixed_uniform_parallel_room(width, source_x) for _ in range(k)]
    )


# Generate a parallel room configuration uniformly at random
def generate_uniform_parallel_room():
    width = random.uniform(0.0, 10.0)
    source_x = random.uniform(0.0, width)
    mic_x = random.uniform(0.0, width)
    mic_y = random.uniform(0.0, 10.0)
    return generate_parallel_room(width, source_x, mic_x, mic_y)


# Generate k parallel room configurations sampled uniformly at random
def generate_uniform_parallel_rooms(k):
    return pl.DataFrame([generate_uniform_parallel_room() for _ in range(k)])


# The samples simulated in each room
samples = pl.DataFrame(
    [
        {"sample": "2400Hz"},
        {"sample": "6000Hz"},
    ]
)

# Describes sources of noise in measurements
# Noise is sampled from a normal distribution with specified standard deviation
# Units are in meters
# noise_mic indicates the noise applied to the actual position of the mic
# noise_walls indicates the noise applied to the actual positions of the
# polygons describing the walls.
# (with actual position, the position used in the simulation is meant)
noise = pl.DataFrame([{"noise_mics": 0.0}]).join(
    pl.DataFrame([{"noise_walls": 0.0}]),
    how="cross",
)

# Describes additional simulation parameters
simulations = pl.DataFrame(
    [
        {"ray_tracing": True, "air_absorption": True, "max_order": 5},
    ]
)


def simulate_room(data):
    print("Working on ", data["id"])
    if data["type"] != "2D" or data["noise_walls"] != 0.0 or data["noise_mics"] != 0.0:
        return []
    fs, audio = wavfile.read(f"Samples/{data['sample']}.wav")
    corners = np.array(data["polygon"]).T
    mics = np.array(data["mics"]).T
    materials = pra.make_materials(*data["materials"])
    max_order = data["max_order"]
    ray_tracing = data["ray_tracing"]
    air_absorption = data["air_absorption"]
    room = pra.Room.from_corners(
        corners,
        fs=fs,
        max_order=max_order,
        mics=mics,
        materials=materials,
        ray_tracing=ray_tracing,
        air_absorption=air_absorption,
        sigma2_awgn=32000.0,
    )

    room.add_source(data["source"], signal=audio, delay=1.0)

    room.compute_rir()

    room.simulate()

    offset = pra.constants.get("frac_delay_length") // 2 + fs
    directory = "Simulated"
    filenames = [f"{data['id']}_mic-{i + 1}.wav" for i in range(len(data["mics"]))]
    for i in range(len(data["mics"])):
        simulated = room.mic_array.signals[i, offset:]
        simulated = np.array(simulated, np.float32)
        simulated /= np.abs(simulated).max()
        wavfile.write(f"{directory}/{filenames[i]}", fs, simulated)

    return filenames


def generate_data():
    rooms = pl.concat(
        [
            generate_fixed_uniform_angle_rooms(0.6, 0.2, 3.0, 10),
            generate_fixed_uniform_angle_rooms(0.3, 0.1, 3.0, 10),
            generate_fixed_uniform_parallel_rooms(10.0, 3.0, 10),
        ]
    )
    # rooms = pl.concat(
    #     [generate_uniform_angle_rooms(1000), generate_uniform_parallel_rooms(1000)]
    # )
    combinations = (
        rooms.join(noise, how="cross")
        .join(simulations, how="cross")
        .join(samples, how="cross")
    )

    # Generate a unique descriptive ID for every row
    combinations = combinations.with_row_index().with_columns(
        pl.concat_str(
            [
                pl.col("room_id"),
                pl.concat_str(
                    [pl.lit("noise"), pl.col("noise_mics"), pl.col("noise_walls")],
                    separator="-",
                ),
                pl.col("sample"),
                pl.when(pl.col("ray_tracing"))
                .then(pl.lit("rt"))
                .otherwise(pl.lit("ism")),
                pl.when(pl.col("air_absorption"))
                .then(pl.lit("aa"))
                .otherwise(pl.lit("naa")),
                pl.col("index"),
            ],
            separator="_",
        ).alias("id")
    )

    # Simulate every room
    combinations = combinations.with_columns(
        pl.struct(combinations.columns)
        .map_elements(simulate_room)
        .alias("simulated_audio")
    )
    combinations.write_json("./generations.json")


if __name__ == "__main__":
    generate_data()
