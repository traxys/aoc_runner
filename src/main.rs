use chrono::Datelike;
use color_eyre::eyre::Context;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};
use structopt::StructOpt;

macro_rules! poss_values {
    ($($value:tt)*) => {
        &[$(stringify!($value),)*]
    };
}

const DAY_EXEC_TEMPLATE: &str = r#"use aoc_2020::{problems::{{day}}::execute, DayContext};

fn main() -> color_eyre::Result<()> {
    let mut context = DayContext::load()?;
    execute(&mut context)?;
    context.report_timings();
    Ok(())
}"#;
const DAY_PROBLEM_STUB: &str = r#"use crate::DayContext;

type Input = ();

pub fn part_1(_: Input) -> color_eyre::Result<String> {
    todo!()
}

pub fn part_2(_: Input) -> color_eyre::Result<String> {
    todo!()
}

pub fn parsing(_: &mut DayContext) -> color_eyre::Result<Input> {
    ()
}

pub fn execute(context: &mut DayContext) -> color_eyre::Result<()> {
    let input = parsing(context);
    context.execute(input, part_1, part_2)
}"#;

#[derive(StructOpt)]
struct Args {
    #[structopt(short, long, possible_values=poss_values!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24))]
    day: Option<u8>,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Run(RunCommand),
    Stub,
}

#[derive(StructOpt)]
struct RunCommand {
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

async fn run(args: RunCommand, day: u8) -> color_eyre::Result<()> {
    let mut data_dir = dirs_next::data_dir().ok_or(color_eyre::eyre::eyre!("No data dir found"))?;
    data_dir.push("aoc_runner.json");

    let data = if !data_dir.exists() {
        let session = promptly::prompt("Your session value")?;
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

    let day_name = format!("day{}", day);
    let mut input = args.input_dir.clone();
    input.push(&day_name);

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

    let executable: PathBuf = format!("src/bin/{}.rs", day_name).into();
    if !executable.exists() {
        let reg = handlebars::Handlebars::new();
        let exec_code = reg
            .render_template(DAY_EXEC_TEMPLATE, &serde_json::json!({ "day": &day_name }))
            .with_context(|| "Could not render day binary template")?;
        let mut exec_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(executable)
            .with_context(|| "Could not open the bin/day file")?;
        exec_file
            .write_all(exec_code.as_bytes())
            .with_context(|| "Could not write the bin/day file")?;
    }

    tokio::process::Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--features",
            &day_name,
            "--bin",
            &day_name,
            "--",
            "--part",
            &format!("{}", args.part),
            "--input",
        ])
        .arg(input)
        .status()
        .await
        .with_context(|| "Could not execute the program")?;

    Ok(())
}

fn stub(day: u8) -> color_eyre::Result<()> {
    let stub = format!("src/problems/day{}.rs", day);
    let mut stub = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(stub)
        .with_context(|| "Could not open day stub file")?;
    stub.write_all(DAY_PROBLEM_STUB.as_bytes())
        .with_context(|| "Could not write day stub")?;

    // We are sure that we don't have the `pub mod dayX` in the problems/mod.rs file
    // because we would have exited on error else
    let mut mod_file = OpenOptions::new()
        .append(true)
        .write(true)
        .create(false)
        .open("src/problems/mod.rs")
        .with_context(|| "Could not open mod file")?;
    mod_file
        .write_all(
            format!(
                "#[cfg(feature = \"day{day}\"]\npub mod day{day};\n",
                day = day
            )
            .as_bytes(),
        )
        .with_context(|| "Could not edit the mod file")?;

    Ok(())
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::from_args();
    let day = args.day.unwrap_or_else(|| chrono::Local::now().day() as u8);

    match args.command {
        Command::Run(command) => run(command, day).await?,
        Command::Stub => stub(day)?,
    }

    Ok(())
}
