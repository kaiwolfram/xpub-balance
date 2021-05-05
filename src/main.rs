use clap::{App, Arg, ArgMatches};

const DEFAULT_OFFSET: &str = "0";
const DEFAULT_N: &str = "20";
const DEFAULT_GAP: &str = "20";

fn main() {
    let matches = App::new("xpub-balance")
        .version("0.1.0")
        .about("Checks the balance of an xpub and its addresses")
        .arg(
            Arg::with_name("xpub")
                .help("Extended public key of your wallet account. Either xpub, ypub or zpub")
                .required(true),
        )
        .arg(
            Arg::with_name("offset")
                .help("Number of addresses to skip")
                .short("o")
                .long("offset")
                .default_value(DEFAULT_OFFSET)
                .validator(is_positive_number)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("n")
            .help("Total number of addresses to check")
            .short("n")
            .default_value(DEFAULT_N)
            .validator(is_positive_number)
            .takes_value(true),
        )
        .arg(
            Arg::with_name("gap")
                .help("Number of subsequently unused addresses to check before terminating the process, also known as gap limit")
                .short("g")
                .long("gap")
                .default_value(DEFAULT_GAP)
                .validator(is_positive_number)
                .takes_value(true),
        )
        .get_matches();
    process_cli(&matches);
}

fn process_cli(matches: &ArgMatches) {
    let xpub = matches.value_of("xpub").unwrap();
    let offset = matches.value_of("offset").unwrap();
    let n = matches.value_of("n").unwrap();
    let gap = matches.value_of("gap").unwrap();

    println!("xpub  : {}", xpub);
    println!("offset: {}", offset);
    println!("n     : {}", n);
    println!("gap   : {}", gap);
}

fn is_positive_number(to_check: String) -> Result<(), String> {
    if to_check.parse::<u32>().is_ok() {
        return Ok(());
    }

    Err(format!("{} is not a valid number", to_check))
}
