use anyhow::{anyhow, Context, Result};
use bdk::database::MemoryDatabase;
use bdk::wallet::AddressIndex;
use bdk::Wallet;
use clap::{App, Arg, ArgMatches};
use esplora_api::blocking::ApiClient;
use esplora_api::data::blockstream::AddressInfoFormat;
use indicatif::{ProgressBar, ProgressStyle};
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;

use libelectrum2descriptors::ElectrumExtendedKey;
use libelectrum2descriptors::ElectrumExtendedPubKey;
use std::str::FromStr;

const DEFAULT_N: &str = "100";
const DEFAULT_START: &str = "0";
const DEFAULT_END: &str = "15";
const DEFAULT_ESPLORA: &str = "https://blockstream.info/api/";

// TODO: add docs
// TODO: split into multiple files
// Test with xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj

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
                .alias("first")
                .help("First index to print")
                .default_value(DEFAULT_START)
                .validator(is_positive_num)
                .index(2)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("end")
                .alias("last")
                .help("Last index to print")
                .default_value(DEFAULT_END)
                .validator(is_positive_num)
                .index(3)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("n")
                .help("Total number of indexes to check. Each index has two addresses: a receive and a change address. Relevant for the total balance calculation")
                .short("n")
                .default_value(DEFAULT_N)
                .validator(is_positive_num)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("esplora")
                .help("Use a specific Esplora URL")
                .short("e")
                .long("esplora")
                .default_value(DEFAULT_ESPLORA)
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

fn process_cli(matches: &ArgMatches) -> Result<()> {
    let args = Args::new(matches)?;
    if args.n <= 0 {
        return Ok(());
    }

    let (receive_descriptor, change_descriptor) = get_descriptors(args.xpub)?;
    let wallet = DerivationWallet {
        receive: create_wallet(&receive_descriptor)?,
        change: create_wallet(&change_descriptor)?,
    };

    if args.is_offline {
        return print_addresses(&wallet, &args);
    }

    let spinner = create_spinner();
    let esplora = ApiClient::new(args.esplora, None)
        .map_err(|_e| anyhow!("Can't connect to {:?}", args.esplora))?;
    check_and_print_addresses(&wallet, &esplora, &args)?;
    spinner.finish_and_clear();

    Ok(())
}

struct Args<'a> {
    xpub: &'a str,
    esplora: &'a str,
    n: u32,
    start: u32,
    end: u32,
    is_change: bool,
    is_offline: bool,
}

impl Args<'_> {
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

struct DerivationWallet {
    receive: Wallet<(), MemoryDatabase>,
    change: Wallet<(), MemoryDatabase>,
}

fn is_positive_num(to_check: String) -> Result<(), String> {
    if to_check.parse::<u32>().is_ok() {
        return Ok(());
    }

    Err(format!("{} is not a valid number", to_check))
}

fn parse_num(to_parse: Option<&str>) -> Result<u32> {
    let value = to_parse.ok_or_else(|| anyhow!("String argument is missing"))?;

    value
        .parse::<u32>()
        .with_context(|| format!("{} is not a valid number", value))
}

fn get_descriptors(xpub: &str) -> Result<(String, String)> {
    let descriptors = ElectrumExtendedPubKey::from_str(xpub)
        .map_err(|e| anyhow!("{:?} is not a valid xpub: {:?}", xpub, e))?
        .to_descriptors();

    let receive = descriptors
        .get(0)
        .ok_or_else(|| {
            anyhow!(
                "Failed to generate descriptor for receiving addresses of {}",
                xpub
            )
        })?
        .to_string();
    let change = descriptors
        .get(1)
        .ok_or_else(|| {
            anyhow!(
                "Failed to generate descriptor for change addresses of {}",
                xpub
            )
        })?
        .to_string();

    Ok((receive, change))
}

fn create_wallet(descriptor: &String) -> Result<Wallet<(), MemoryDatabase>> {
    Ok(Wallet::new_offline(
        descriptor,
        None,
        bdk::bitcoin::Network::Bitcoin,
        MemoryDatabase::default(),
    )?)
}

// TODO: use async instead of rayon
fn check_and_print_addresses(
    wallet: &DerivationWallet,
    esplora: &ApiClient,
    args: &Args<'_>,
) -> Result<()> {
    let (receive, change) = derive_addresses(wallet, args)?;

    let receive_info = request_esplora(esplora, &receive)?;
    let change_info = request_esplora(esplora, &change)?;

    print_address_infos(&receive_info, &change_info, &args)?;

    let (total_balance, total_txs) = calculate_totals(&receive_info, &change_info);

    print_summary(total_balance, total_txs);

    Ok(())
}

fn print_summary(balance: i64, tx_count: i32) {
    println!(
        "\n-> total balance     : {} sat\n-> total transactions: {} txs",
        balance.to_formatted_string(&Locale::en),
        tx_count.to_formatted_string(&Locale::en)
    );
}

fn print_address_info(
    index: u32,
    is_change: bool,
    address: &str,
    balance: Option<i64>,
    tx_count: Option<i32>,
) {
    let full_index = format!("{}/{}", if is_change { 1 } else { 0 }, index);
    print!("{:<5} {:<40}", full_index, address);
    if let Some(num) = balance {
        print!("  {} sat", num.to_formatted_string(&Locale::en));
    }
    if let Some(num) = tx_count {
        print!("  {} txs", num.to_formatted_string(&Locale::en));
    }
    println!();
}

fn print_addresses(wallet: &DerivationWallet, args: &Args) -> Result<()> {
    let to_print = if args.is_change {
        &wallet.change
    } else {
        &wallet.receive
    };

    for i in args.start..=args.end {
        let address = to_print.get_address(AddressIndex::Peek(i))?;
        print_address_info(i, args.is_change, &address.to_string(), None, None)
    }

    Ok(())
}

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

fn derive_addresses(wallet: &DerivationWallet, args: &Args) -> Result<(Vec<String>, Vec<String>)> {
    let mut receive: Vec<String> = Vec::with_capacity(args.n as usize);
    let mut change: Vec<String> = Vec::with_capacity(args.n as usize);

    for i in 0..args.n {
        let receive_address = wallet.receive.get_address(AddressIndex::Peek(i))?;
        receive.push(receive_address.to_string());
        let change_address = wallet.change.get_address(AddressIndex::Peek(i))?;
        change.push(change_address.to_string());
    }

    Ok((receive, change))
}

fn request_esplora(esplora: &ApiClient, addresses: &Vec<String>) -> Result<Vec<AddressInfoFormat>> {
    let results = addresses
        .par_iter()
        .map(|addr| {
            esplora
                .get_address(addr)
                .map_err(|_e| anyhow!("Esplora request for {:?} failed", addr))
        })
        .collect::<Vec<Result<AddressInfoFormat>>>();

    let mut address_infos: Vec<AddressInfoFormat> = Vec::with_capacity(results.len());
    for addr in results.into_iter() {
        address_infos.push(addr?);
    }

    Ok(address_infos)
}

trait AddressInfo {
    fn balance(&self) -> i64;
    fn tx_count(&self) -> i32;
    fn address(&self) -> Result<&String>;
}

impl AddressInfo for AddressInfoFormat {
    fn balance(&self) -> i64 {
        self.chain_stats.funded_txo_sum - self.chain_stats.spent_txo_sum
    }
    fn tx_count(&self) -> i32 {
        self.chain_stats.tx_count
    }
    fn address(&self) -> Result<&String> {
        Ok(self
            .address
            .as_ref()
            .ok_or_else(|| anyhow!("Missing address in Esplora response"))?)
    }
}

fn calculate_totals(
    receive: &Vec<AddressInfoFormat>,
    change: &Vec<AddressInfoFormat>,
) -> (i64, i32) {
    let (balance, tx_count): (Vec<i64>, Vec<i32>) = receive
        .iter()
        .chain(change.iter())
        .map(|i| (i.balance(), i.tx_count()))
        .unzip();

    (balance.iter().sum(), tx_count.iter().sum())
}

fn print_address_infos(
    receive_info: &Vec<AddressInfoFormat>,
    change_info: &Vec<AddressInfoFormat>,
    args: &Args,
) -> Result<()> {
    let to_print = if args.is_change {
        change_info
    } else {
        receive_info
    };

    let mut index = args.start;
    for addr in to_print {
        print_address_info(
            index,
            args.is_change,
            addr.address()?,
            Some(addr.balance()),
            Some(addr.tx_count()),
        );
        index += 1;

        if index > args.end {
            break;
        }
    }

    Ok(())
}
