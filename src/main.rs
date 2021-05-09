mod constants;
mod esplora;
mod print;
mod wallet;

use anyhow::{anyhow, Context, Result};

use clap::{App, Arg, ArgMatches};
use indicatif::{ProgressBar, ProgressStyle};

use esplora::*;
use print::*;

use wallet::*;

fn main() -> Result<()> {
    let matches = App::new("xpub-balance")
        .version("0.1.0")
        .about("Check the balance of an xpub and its addresses")
        .arg(
            Arg::with_name("xpub")
                .help("Extended public key of your wallet account. Either xpub, ypub or zpub")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("start")
                .help("First index to print")
                .default_value(constants::DEFAULT_START)
                .validator(is_positive_num)
                .index(2)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("end")
                .help("Last index to print")
                .default_value(constants::DEFAULT_END)
                .validator(is_positive_num)
                .index(3)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("n")
                .help("Total number of indexes to check. Each index has two addresses: a receive and a change address. Relevant for the total balance calculation")
                .short("n")
                .default_value(constants::DEFAULT_N)
                .validator(is_positive_num)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("esplora")
                .alias("url")
                .help("Use a specific Esplora URL")
                .short("e")
                .long("esplora")
                .default_value(constants::DEFAULT_ESPLORA)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("change")
                .help("Show change instead of receive addresses. Doesn't effect the total balance calculation")
                .short("c")
                .long("change")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("offline")
                .help("Only show addresses. No reqests will be send")
                .short("o")
                .long("offline")
                .takes_value(false),
        )
        .get_matches();
    process_cli(&matches)
}

/// Processes command line arguments and executes program
fn process_cli(matches: &ArgMatches) -> Result<()> {
    let args = Args::new(matches)?;
    if args.n <= 0 {
        return Ok(());
    }

    let wallet = DerivationWallet::new(args.xpub)?;

    if args.is_offline {
        return print_addresses_offline(&wallet, &args);
    }

    let spinner = create_spinner();
    let (receive_info, change_info) = request_data(&wallet, &args)?;
    spinner.finish_and_clear();

    return print_all(&receive_info, &change_info, &args);
}

/// Holds the command line arguments
pub struct Args<'a> {
    xpub: &'a str,
    esplora: &'a str,
    n: u32,
    start: u32,
    end: u32,
    is_change: bool,
    is_offline: bool,
}

impl Args<'_> {
    /// Reads command line arguments and saves them in a new struct
    fn new<'a>(matches: &'a ArgMatches) -> Result<Args<'a>> {
        let start = parse_num(matches.value_of("start"))?;
        let mut end = parse_num(matches.value_of("end"))?;

        if end < start {
            end = start;
        }

        Ok(Args {
            xpub: matches
                .value_of("xpub")
                .with_context(|| "xpub is missing")?,
            esplora: matches
                .value_of("esplora")
                .with_context(|| "Esplora URL is missing")?,
            n: parse_num(matches.value_of("n"))?,
            start,
            end,
            is_change: matches.is_present("change"),
            is_offline: matches.is_present("offline"),
        })
    }
}

/// Checks if a string is a [`u32`]
fn is_positive_num(to_check: String) -> Result<(), String> {
    if to_check.parse::<u32>().is_ok() {
        return Ok(());
    }

    Err(format!("{} is not a valid number", to_check))
}

/// Parses a string to a [`u32`]
fn parse_num(to_parse: Option<&str>) -> Result<u32> {
    let value = to_parse.ok_or_else(|| anyhow!("String argument is missing"))?;

    value
        .parse::<u32>()
        .with_context(|| format!("{} is not a valid number", value))
}

/// Creates a spinner to indicate that the program is still running. Shows "Requesting data..." with moving dots
fn create_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(240);
    spinner.set_message("Requesting data");
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[".  ", ".. ", "...", "   "])
            .template("{elapsed_precise} {msg}{spinner}"),
    );

    spinner
}
