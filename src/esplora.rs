use super::Args;
use crate::wallet::{derive_addresses, DerivationWallet};
use anyhow::{anyhow, Result};
use esplora_api::blocking::ApiClient;
use esplora_api::data::blockstream::AddressInfoFormat;
use rayon::prelude::*;

/// Requests wallet data from Esplora
pub fn request_data(
    wallet: &DerivationWallet,
    args: &Args,
) -> Result<(Vec<AddressInfoFormat>, Vec<AddressInfoFormat>)> {
    let esplora = ApiClient::new(args.esplora, None)
        .map_err(|_e| anyhow!("Can't connect to {:?}", args.esplora))?;
    let (receive, change) = derive_addresses(&wallet, &args)?;
    let receive_info = request_esplora(&esplora, &receive)?;
    let change_info = request_esplora(&esplora, &change)?;

    Ok((receive_info, change_info))
}

/// Trait to easily calculate balance and transaction count of a [`AddressInfoFormat`]
pub trait AddressInfo {
    fn balance(&self) -> i64;
    fn tx_count(&self) -> i32;
    fn address(&self) -> Result<&String>;
}

impl AddressInfo for AddressInfoFormat {
    /// Calculate and return the balance
    fn balance(&self) -> i64 {
        self.chain_stats.funded_txo_sum - self.chain_stats.spent_txo_sum
    }
    /// Return the transaction count
    fn tx_count(&self) -> i32 {
        self.chain_stats.tx_count
    }
    /// Return the address
    fn address(&self) -> Result<&String> {
        Ok(self
            .address
            .as_ref()
            .ok_or_else(|| anyhow!("Missing address in Esplora response"))?)
    }
}

/// Calculates the sum of all balances and transaction counts of a list of [`AddressInfoFormat`]
/// and returns it as a tuple.
/// First item is the total balance and the second one the total transaction count
pub fn calculate_totals(
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

// TODO: use async instead of rayon
/// Sends address requests to Esplora and returns the responses
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
