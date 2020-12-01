use chrono::Datelike;
use color_eyre::eyre::Context;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    time::Duration,
    time::Instant,
};
use structopt::StructOpt;

macro_rules! poss_values {
    ($($value:tt)*) => {
        &[$(stringify!($value),)*]
    };
}

#[derive(StructOpt)]
struct Args {
    #[structopt(short, long, possible_values=poss_values!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24))]
    day: Option<u8>,
    #[structopt(short, long)]
    year: Option<u16>,
    #[structopt(short, long, default_value="1", possible_values=&["1", "2"])]
    part: u8,
    #[structopt(short, long, default_value = "inputs")]
    input_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Data {
    session: String,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::from_args();

    let mut data_dir = dirs_next::data_dir().ok_or(color_eyre::eyre::eyre!("No data dir found"))?;
    data_dir.push("aoc_runner.json");

    let data = if !data_dir.exists() {
        let session = promptly::prompt("Your session value:")?;
        let d = Data { session };
        serde_json::to_writer_pretty(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&data_dir)
                .with_context(|| format!("Could not open data file at {:?}", data_dir))?,
            &d,
        )
        .with_context(|| "Could not serialize data")?;
        d
    } else {
        serde_json::from_reader(
            File::open(&data_dir)
                .with_context(|| format!("Could not open data file at {:?}", data_dir))?,
        )
        .with_context(|| "Could not read data file")?
    };

    let day = args.day.unwrap_or_else(|| chrono::Local::now().day() as u8);

    let mut input = args.input_dir;
    input.push(format!("day{}", day));

    if !input.exists() {
        let year = args
            .year
            .unwrap_or_else(|| chrono::Local::now().year() as u16);

        let client = reqwest::Client::new();
        let body = client
            .get(&format!(
                "https://adventofcode.com/{}/day/{}/input",
                year, day
            ))
            .header("Cookie", format!("session={}", data.session))
            .send()
            .await
            .with_context(|| format!("Could not fetch the input for day {} of AoC {}", day, year))?
            .error_for_status()
            .with_context(|| format!("Error accessing the input for day {} of AoC {}", day, year))?
            .text()
            .await
            .with_context(|| "Error reading the body of the response")?;

        let mut writer = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&input)
            .with_context(|| format!("Could not open file at {:?}", input))?;

        writer
            .write_all(body.as_bytes())
            .with_context(|| format!("Could not write to file {:?}", input))?;
    }

    let now = Instant::now();
    tokio::process::Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "--part",
            &format!("{}", args.part),
            "--input",
        ])
        .arg(input)
        .status()
        .await
        .with_context(|| "Could not execute the program")?;
    let elapsed = now.elapsed().as_nanos() as f64;

    print!("Time taken: ");

    let secs_ns = (10.0f64).powi(9);
    let ms_ns = (10.0f64).powi(6);
    let us_ns = (10.0f64).powi(3);
    if elapsed > secs_ns {
        println!("{:02}s", elapsed / secs_ns);
    } else if elapsed > ms_ns {
        println!("{:02}ms", elapsed / ms_ns)
    } else if elapsed > us_ns {
        println!("{:02}us", elapsed / us_ns)
    } else {
        println!("{:02}ns", elapsed)
    }

    Ok(())
}
