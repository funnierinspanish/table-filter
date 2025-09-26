# Table Filter

Table Filter is a simple program for filtering data in a table-like structure from `stdin`. It allows you to filter rows based on the values in specific columns.

## Features

- Filter rows by column values (by name or index)
- Select and print specific columns
- Sort rows by any column (ascending/descending)
- Transform column values (date conversion only for now)
- Save and load profiles for common settings

## Installation

### Option 1: Install from Cargo

```bash
cargo install --git https://github.com/funnierinspanish/table-filter.git
```

### Option 2: Download Pre-built Binaries

Download the latest release for your platform from the [GitHub releases page](https://github.com/funnierinspanish/table-filter/releases):


**Linux (x86_64):**

```bash
# Download and extract
curl -L https://github.com/funnierinspanish/table-filter/releases/download/v0.3.0/tf
chmod +x ./tf
sudo mv tf ~/./local/bin/tf
# or depending on your system:
#sudo mv tf /usr/local/bin/tf
```

**macOS (Apple Silicon, M1/M2):**

```bash
curl -L https://github.com/funnierinspanish/table-filter/releases/latest/download/tf-aarch64-apple-darwin.tar.gz | tar xz
chmod +x tf-aarch64-apple-darwin
sudo mv tf-aarch64-apple-darwin /usr/local/bin/tf
```

**Windows:**

1. Download `tf-x86_64-pc-windows-msvc.zip` from the [releases page](https://github.com/funnierinspanish/table-filter/releases)
2. Extract the ZIP file
3. Move `tf.exe` to a directory in your PATH

### Option 3: Build from Source

```bash
git clone https://github.com/funnierinspanish/table-filter.git
cd table-filter
cargo build --release && cargo install --path .
# The binary will be in ~/.cargo/bin/tf
```

This will install the binary as `tf` in `~/.local/bin/` (You can also specify a `DEST` and `NEWNAME` variable).

## Basic Usage

Example file [test_data.txt](./test_data.txt):

```txt
 Name     │ Apartment Number │ Roommate Count  │ Preferred Lunch  │ Got preferred lunch  │ Last time eaten
----------+------------------+----------------+------------------+-----------------------+----------------
 george   │ 202              │  0             │ soup             │ failed                │ 5d
 jerry    │ 304              │  1             │ soup             │ failed                │ 15d
 the guy  │ 101              │  2             │ soup             │ succeeded             │ 2m
```

Filter and print rows where the first column contains "george":

```bash
cat test_data.txt | tf --cols "Name,Apartment Number" --match '{"Name": "george"}'
```


Print only specific columns:

```bash
cat test_data.txt | tf --cols "Name,Preferred Lunch"
```

Sort by a column:

```bash
cat test_data.txt | tf --cols "Name,Last time eaten" --sort-by "Last time eaten" --sort-order desc
```

Apply a transformation:

```bash
cat test_data.txt | tf --cols "Name" --transform '{"Last time eaten": "$AGE_TO_DATE"}'
```

Skip the first 2 lines of input and show results without headers:

```bash
cat test_data.txt | tf --skip-lines 2 --cols "Name,Preferred Lunch" --no-headers
```

Skip the first result after filtering/sorting:

```bash
cat test_data.txt | tf --cols "Name,Last time eaten" --sort-by "Last time eaten" --skip-results 1 --sort-order desc
```

## Arguments


- `--headers-row <N>`: Row number (1-based) containing column headers (**required**)
- `-p, --profile <PROFILE_NAME>`: Use a saved profile with predefined settings in `~/.config/table-formatter.config.json`
- `--cols <cols>`: Comma-separated list of columns to print (by name or `$N` for index)
- `--match <json>`: JSON object mapping columns to values to filter (supports arrays)
- `--sort-by <col>`: Column to sort by
- `--sort-order <asc|desc>`: Sort order (default: asc)
- `--transform <json>`: JSON object mapping columns to transformations (`$AGE_TO_DATE`)
- `--separator <sep>`: Column separator (default: `│`)
- `--skip-lines <N>`: Skip first N lines of input
- `--skip-results <N>`: Skip first N results after filtering and sorting
- `--no-headers`: Don't print column headers in output

## Profiles

You can save and load profiles for common settings.

### Create or update a profile

```bash
tf config set myprofile.headers-row=1
tf config set myprofile.cols='["Name","Last time eaten"]'
```

### Use a profile

```bash
cat test_data.txt | tf --profile myprofile
```

### View a profile

```bash
tf config get --profile myprofile
```

## Configuration

Profiles are stored in `~/.config/tf.config.json`.

## Transformations

- `$AGE_TO_DATE`: Convert age string like `15d` to a date

## Example: Full Command

```bash
cat test_data.txt | tf --cols '"$1","Got preferred lunch","$6"' --match '{"Got preferred lunch": "succeeded"}' --sort-by '$6' --sort-order desc
```

---

For more, run:

```bash
tf --help
```
