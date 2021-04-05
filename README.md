## A Fast osu! pp calculator web api written in Rust

It is also the PP server of **[Peace](https://github.com/Pure-Peace/peace)**.

**Pure-Rust** pp calculator based on [peace-performance](https://github.com/Pure-Peace/peace-performance) and [rosu-pp](https://github.com/MaxOhn/rosu-pp).

### Features

- **Common**:
  - **request with you like: beatmap md5, beatmap id, beatmapset id + file name**
  - **beatmap cache**
  - **preload beatmaps** (WARING: May cause insufficient memory, if the number of maps is large enough)
  - **calculate beatmap MD5**
  - **auto request, download beatmap from osu!api**
  - **Oppai? Or a custom algorithm**
  - **pp calculate**:
    - osu!Standard
    - Catch the beat
    - Taiko
    - Mainia
- Enable feature **peace**:
  - beatmap database (needs setup with [Peace](https://github.com/Pure-Peace/Peace/tree/main/sql))
  - How to enable?
    - `Cargo.toml` Set `default = []` => `default = ["peace"]`
  
### Best performance (Fastest, but lower accuracy)

Set Cargo.toml

```rust
peace-performance = { git = "https://github.com/Pure-Peace/Peace-performance.git", branch = "main" }
```

to

```rust
peace-performance = { git = "https://github.com/Pure-Peace/Peace-performance.git", branch = "main", feature = "no_sliders_no_leniency" }
```

## Note

**This pp-server requires all `.osu` files use file MD5 as the name.**

- **Rename .osu files to file md5:**

  - If you want **pp server** auto recalculate all `.osu` files MD5 name before started, set `recalculate_osu_file_md5 = true` in `pp-server-config/default.toml`
  - Or manual run this python script `rename_osu_files.py` in project (python3.8+).
  - If its Debug compile, python will more faster than Rust.

- **Effect**
  - Calculating
  - ![p](screenshot/ef1.png)
  - After
  - ![p](screenshot/ef2.png)

### Setup

1. Set your `.osu` files dir path in `pp-server-config/default.toml`
2. Will let the `.osu` files name be the `md5` of the file
3. Set your osu!api keys in *.toml (if enabled feature `peace`, set it on your database)

### Debug

```
cargo run
```

### Release

```
cargo run --release
```

**Cross compile (Win to Linux)**

```
cargo cross_linux_x86
```

## MIT
