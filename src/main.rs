use anyhow::{anyhow, Context, Result};
use bdk::database::MemoryDatabase;
use bdk::wallet::AddressIndex;
use bdk::Wallet;
use clap::{App, Arg, ArgMatches};
use esplora_api::blocking::ApiClient;
use esplora_api::data::blockstream::AddressInfoFormat;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use libelectrum2descriptors::ElectrumExtendedKey;
use libelectrum2descriptors::ElectrumExtendedPubKey;
use std::str::FromStr;

const DEFAULT_N: &str = "100";
const DEFAULT_START: &str = "0";
const DEFAULT_END: &str = "15";
const DEFAULT_ESPLORA: &str = "https://blockstream.info/api/";

// TODO: no unwrap()
// TODO: add docs
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
    let (receive_descriptor, change_descriptor) = get_descriptors(args.xpub)?;
    let wallet = DerivationWallet {
        receive: create_wallet(&receive_descriptor)?,
        change: create_wallet(&change_descriptor)?,
    };

    if args.is_offline {
        return print_addresses(&wallet, &args);
    }

    let esplora = ApiClient::new(args.esplora, None).unwrap();

    return check_and_print_addresses(&wallet, &esplora, &args);
}

struct DerivationWallet {
    receive: Wallet<(), MemoryDatabase>,
    change: Wallet<(), MemoryDatabase>,
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
            xpub: matches.value_of("xpub").unwrap(),
            esplora: matches.value_of("esplora").unwrap(),
            n: parse_num(matches.value_of("n"))?,
            start,
            end,
            is_change: matches.is_present("change"),
            is_offline: matches.is_present("offline"),
        })
    }
}

fn is_positive_num(to_check: String) -> Result<(), String> {
    if to_check.parse::<u32>().is_ok() {
        return Ok(());
    }

    Err(format!("{} is not a valid number", to_check))
}

fn parse_num(to_parse: Option<&str>) -> Result<u32> {
    to_parse
        .unwrap()
        .parse::<u32>()
        .with_context(|| format!("{} is not a valid number", to_parse.unwrap()))
}

fn get_descriptors(xpub: &str) -> Result<(String, String)> {
    let descriptors = ElectrumExtendedPubKey::from_str(xpub)
        .map_err(|e| anyhow!("{:?} is not a valid xpub: {:?}", xpub, e))?
        .to_descriptors();

    Ok((
        descriptors.get(0).unwrap().to_string(),
        descriptors.get(1).unwrap().to_string(),
    ))
}

fn create_wallet(descriptor: &String) -> Result<Wallet<(), MemoryDatabase>> {
    Ok(Wallet::new_offline(
        descriptor,
        None,
        bdk::bitcoin::Network::Bitcoin,
        MemoryDatabase::default(),
    )?)
}

// TODO: refactor
// TODO: pretty print
// TODO: use async instead of rayon
fn check_and_print_addresses(
    wallet: &DerivationWallet,
    esplora: &ApiClient,
    args: &Args<'_>,
) -> Result<()> {
    let (receive, change) = derive_addresses(wallet, args)?;

    let spinner = create_spinner();
    let (receive_info, change_info) = request_esplora(esplora, &receive, &change)?;
    spinner.finish_and_clear();

    print_address_infos(&receive_info, &change_info, &args);

    let (total_balance, total_txs) = calculate_totals(&receive_info, &change_info);
    println!();
    println!("-> total balance     : {} sat", total_balance);
    println!("-> total transactions: {} txs", total_txs);

    Ok(())
}

fn print_addresses(wallet: &DerivationWallet, args: &Args) -> Result<()> {
    for i in 0..args.n {
        let address = wallet.receive.get_address(AddressIndex::Peek(i))?;
        if !args.is_change && (args.start..=args.end).contains(&i) {
            println!("{} {}", i, address);
        }

        let address = wallet.change.get_address(AddressIndex::Peek(i))?;
        if args.is_change && (args.start..=args.end).contains(&i) {
            println!("{} {}", i, address);
        }
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

fn request_esplora(
    esplora: &ApiClient,
    receive: &Vec<String>,
    change: &Vec<String>,
) -> Result<(Vec<AddressInfoFormat>, Vec<AddressInfoFormat>)> {
    let receive_info = receive
        .par_iter()
        .map(|addr| esplora.get_address(addr).unwrap())
        .collect::<Vec<AddressInfoFormat>>();
    let change_info = change
        .par_iter()
        .map(|addr| esplora.get_address(addr).unwrap())
        .collect::<Vec<AddressInfoFormat>>();

    Ok((receive_info, change_info))
}

fn calculate_totals(
    receive: &Vec<AddressInfoFormat>,
    change: &Vec<AddressInfoFormat>,
) -> (i64, i32) {
    let (balance, tx): (Vec<i64>, Vec<i32>) = receive
        .iter()
        .chain(change.iter())
        .map(|i| {
            (
                i.chain_stats.funded_txo_sum - i.chain_stats.spent_txo_sum,
                i.chain_stats.tx_count,
            )
        })
        .unzip();

    (balance.iter().sum(), tx.iter().sum())
}

fn print_address_infos(
    receive_info: &Vec<AddressInfoFormat>,
    change_info: &Vec<AddressInfoFormat>,
    args: &Args,
) {
    let to_print = if args.is_change {
        change_info
    } else {
        receive_info
    };

    let mut index = args.start;
    for addr in to_print {
        let balance = addr.chain_stats.funded_txo_sum - addr.chain_stats.spent_txo_sum;
        let txs = addr.chain_stats.tx_count;
        println!(
            "{} {} {} sat {} txs",
            index,
            addr.address.as_ref().unwrap(),
            balance,
            txs
        );
        index += 1;

        if index > args.end {
            break;
        }
    }
}
