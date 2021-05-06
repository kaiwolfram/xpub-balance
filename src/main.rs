use anyhow::{anyhow, Context, Result};
use bdk::database::MemoryDatabase;
use bdk::wallet::AddressIndex;
use bdk::Wallet;
use clap::{App, Arg, ArgMatches};
use esplora_api::blocking::ApiClient;
use libelectrum2descriptors::ElectrumExtendedKey;
use libelectrum2descriptors::ElectrumExtendedPubKey;
use std::str::FromStr;

const DEFAULT_N: &str = "100";
const DEFAULT_SHOW: &str = "15";
const DEFAULT_ESPLORA: &str = "https://blockstream.info/api/";

// TODO: no unwrap()
// TODO: add docs
// TODO: add flag to use own esplora
// Test with xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj
fn main() -> Result<()> {
    let matches = App::new("xpub-balance")
        .version("0.1.0")
        .about("Check the balance of an xpub and its addresses")
        .arg(
            Arg::with_name("xpub")
                .help("Extended public key of your wallet account. Either xpub, ypub or zpub")
                .required(true),
        )
        .arg(
            Arg::with_name("show")
                .help("Total number of addresses to print to the console")
                .short("s")
                .long("show")
                .default_value(DEFAULT_SHOW)
                .validator(is_positive_num)
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
            Arg::with_name("n")
                .help("Total number of indexes to check. Each index has two addresses: a receive and a change address. Relevant for the total balance calculation")
                .short("n")
                .default_value(DEFAULT_N)
                .validator(is_positive_num)
                .takes_value(true),
        )
        .get_matches();
    process_cli(&matches)
}

// TODO: Refactor
fn process_cli(matches: &ArgMatches) -> Result<()> {
    let xpub = matches.value_of("xpub").unwrap();
    let n = parse_num(matches.value_of("n"))?;
    let show = parse_num(matches.value_of("show"))?;
    let isChange = matches.is_present("change");

    let descriptors = ElectrumExtendedPubKey::from_str(xpub)
        .map_err(|e| anyhow!("{:?} is not a valid xpub: {:?}", xpub, e))?
        .to_descriptors();

    let wallet = Wallet::new_offline(
        descriptors.get(0).unwrap(),
        Some(descriptors.get(1).unwrap()),
        bdk::bitcoin::Network::Bitcoin,
        MemoryDatabase::default(),
    )?;

    let esplora = ApiClient::new(DEFAULT_ESPLORA, None).unwrap();

    // TODO: Check and print change addresses
    // TODO: Improve performance
    let mut total_balance = 0;
    let mut total_txs = 0;
    for i in 0..n {
        let address = wallet.get_address(AddressIndex::Peek(i))?;
        let info = esplora.get_address(&address.to_string()).unwrap();
        let balance = info.chain_stats.funded_txo_sum - info.chain_stats.spent_txo_sum;
        let tx = info.chain_stats.tx_count;
        total_balance += balance;
        total_txs += tx;

        if i < show {
            println!("{} {} {}sat {}txs", i, address, balance, tx);
        }
    }

    println!("total balance: {}", total_balance);
    println!("total transactions: {}", total_txs);

    return Ok(());
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
