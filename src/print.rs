use super::Args;
use crate::esplora::*;
use crate::wallet::DerivationWallet;
use anyhow::Result;
use bdk::wallet::AddressIndex;
use esplora_api::data::blockstream::AddressInfoFormat;
use num_format::{Locale, ToFormattedString};

/// Prints desired addresses with their infos and the summary
pub fn print_all(
    receive_info: &Vec<AddressInfoFormat>,
    change_info: &Vec<AddressInfoFormat>,
    args: &Args,
) -> Result<()> {
    print_address_infos(&receive_info, &change_info, &args)?;
    let (total_balance, total_txs) = calculate_totals(&receive_info, &change_info);
    print_summary(total_balance, total_txs);

    Ok(())
}

/// Derives and prints addresses of the desired account with their indexes
pub fn print_addresses_offline(wallet: &DerivationWallet, args: &Args) -> Result<()> {
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

/// Prints the summary of total balances and transaction counts
fn print_summary(balance: i64, tx_count: i32) {
    println!(
        "\n-> total balance     : {} sat\n-> total transactions: {} txs\n",
        balance.to_formatted_string(&Locale::en),
        tx_count.to_formatted_string(&Locale::en)
    );
}

/// Print a single address of the desired account with its index, balance and transaction count
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

/// Prints the addresses of the desired account with their indexes, balances and transaction counts
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
