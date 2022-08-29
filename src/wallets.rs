use std::collections::HashMap;

use super::Result;
use bincode::{deserialize, serialize};
use bitcoincash_addr::{Address, HashType, Scheme};
use crypto::{digest::Digest, ed25519, ripemd160::Ripemd160, sha2::Sha256};
use log::info;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Wallet {
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Wallet {
    pub fn new() -> Self {
        let mut key: [u8; 64] = [0; 64];
        OsRng.fill_bytes(&mut key);
        let (secret_key, public_key) = ed25519::keypair(&key);
        let secret_key = secret_key.to_vec();
        let public_key = public_key.to_vec();

        Wallet {
            secret_key,
            public_key,
        }
    }

    fn get_address(&self) -> String {
        let mut pub_hash: Vec<u8> = self.public_key.clone();
        hash_pub_key(&mut pub_hash);

        let address = Address {
            body: pub_hash,
            scheme: Scheme::Base58,
            hash_type: HashType::Script,
            ..Default::default()
        };

        address.encode().unwrap()
    }
}

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn new() -> Result<Wallets> {
        let mut wlts = Wallets {
            wallets: HashMap::<String, Wallet>::new(),
        };

        let db = sled::open("data/wallets")?;
        for item in db.into_iter() {
            let i = item?;
            let address = String::from_utf8(i.0.to_vec())?;
            let wallet = deserialize(&i.1.to_vec())?;
            wlts.wallets.insert(address, wallet);
        }

        Ok(wlts)
    }

    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        info!("create wallet: {}", address);

        address
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    pub fn get_all_addresses(&self) -> Vec<String> {
        let mut addresses = Vec::<String>::new();

        for (address, _) in &self.wallets {
            addresses.push(address.clone());
        }

        addresses
    }

    pub fn save_all(&self) -> Result<()> {
        let db = sled::open("data/wallets")?;

        for (address, wallet) in &self.wallets {
            let data = serialize(&wallet)?;
            db.insert(address, data)?;
        }

        db.flush()?;
        Ok(())
    }
}

pub fn hash_pub_key(pub_key: &mut Vec<u8>) {
    let mut hasher_01 = Sha256::new();
    hasher_01.input(pub_key);
    hasher_01.result(pub_key);

    let mut hasher_02 = Ripemd160::new();
    hasher_02.input(pub_key);
    pub_key.resize(20, 0);
    hasher_02.result(pub_key);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_wallet_and_hash() {
        let w1 = Wallet::new();
        let w2 = Wallet::new();
        assert_ne!(w1, w2);
        assert_ne!(w1.get_address(), w2.get_address());

        let mut p2 = w2.public_key.clone();
        hash_pub_key(&mut p2);
        assert_eq!(p2.len(), 20);

        let pub_key_hash = Address::decode(&w2.get_address()).unwrap().body;
        assert_eq!(pub_key_hash, p2);
    }

    #[test]
    fn test_wallets() {
        let mut wlts = Wallets::new().unwrap();
        let wlt_address = wlts.create_wallet();
        let wlt1 = wlts.get_wallet(&wlt_address).unwrap().clone();
        wlts.save_all().unwrap();
        drop(wlts);

        let wlts2 = Wallets::new().unwrap();
        let wlt2 = wlts2.get_wallet(&wlt_address).unwrap();
        assert_eq!(&wlt1, wlt2);
    }

    #[test]
    #[should_panic]
    fn test_wallets_not_exist() {
        let mut wlts = Wallets::new().unwrap();
        wlts.create_wallet();
        wlts.save_all().unwrap();
        drop(wlts);

        let wlt = Wallet::new();
        let wlts2 = Wallets::new().unwrap();
        wlts2.get_wallet(&wlt.get_address()).unwrap();
    }
}
