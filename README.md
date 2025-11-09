# Localization from all echoes of a fixed sound source in a convex 2D two-wall environment

This repository contains the code to generate simulated acoustic data in 2D environments, and the code used to localize a microphone from a (simulated) recorded signal and knowledge of the room and sound source.

## Dataset generation
The file `Generation/main.py` contains the code for generating a dataset. To generate a dataset, for a specific set of rooms, add the room configurations to the `rooms` variable at the start of `generate_data`.

The generate the data, run the following from the root of the repository.
```
python Generation/main.py
```
The python code itself depends on `numpy`, `polars`, `scipy` and `pyroomacoustics`. These dependencies are embedded in the Nix shell provided by the `flake.nix`.

## Localization
The directory `Localization` contains a rust project for localization from a sound signal.
In order to perform localization, run the following from the root directory after generating a dataset.
```
cargo run --manifest-path Localization/Cargo.toml --release
```