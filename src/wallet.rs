use super::Args;
use anyhow::{anyhow, Result};
use bdk::database::MemoryDatabase;
use bdk::wallet::AddressIndex;
use bdk::Wallet;
use libelectrum2descriptors::ElectrumExtendedKey;
use libelectrum2descriptors::ElectrumExtendedPubKey;
use std::str::FromStr;

/// Holds two [`Wallet`], one for the receive and the other for the change account.
/// [`Wallet`] doesn't allow us to derive change addresses, that's why we use a separate [`Wallet`] for the change account
pub struct DerivationWallet {
    pub receive: Wallet<(), MemoryDatabase>,
    pub change: Wallet<(), MemoryDatabase>,
}

impl DerivationWallet {
    /// Creates a new derivation wallet
    pub fn new(xpub: &str) -> Result<DerivationWallet> {
        let (receive_descriptor, change_descriptor) = get_descriptors(xpub)?;

        Ok(DerivationWallet {
            receive: create_wallet(&receive_descriptor)?,
            change: create_wallet(&change_descriptor)?,
        })
    }
}

/// Creates a new [`Wallet`] without a change account
pub fn create_wallet(descriptor: &String) -> Result<Wallet<(), MemoryDatabase>> {
    Ok(Wallet::new_offline(
        descriptor,
        None,
        bdk::bitcoin::Network::Bitcoin,
        MemoryDatabase::default(),
    )?)
}

/// Derives receive and change addresses
pub fn derive_addresses(
    wallet: &DerivationWallet,
    args: &Args,
) -> Result<(Vec<String>, Vec<String>)> {
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

/// Returns the receive and change account descriptors of a xpub
pub fn get_descriptors(xpub: &str) -> Result<(String, String)> {
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
