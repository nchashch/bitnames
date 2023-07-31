use ddk::heed;
use ddk::node::State;
use ddk::types::{GetValue, Hash, Transaction};
use heed::{types::*, Database};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BitName {
    KeyValue { key: Hash, value: Hash },
}

impl GetValue for BitName {
    fn get_value(&self) -> u64 {
        0
    }
}

#[derive(Clone)]
pub struct BitNamesState {
    key_to_value: Database<OwnedType<Hash>, OwnedType<Hash>>,
}

impl BitNamesState {
    fn validate_keys_unique(
        &self,
        txn: &heed::RoTxn,
        transaction: &Transaction<BitName>,
    ) -> Result<(), Error> {
        for output in &transaction.outputs {
            match output.content {
                ddk::types::Content::Custom(BitName::KeyValue { key, .. }) => {
                    if self.key_to_value.get(txn, &key)?.is_some() {
                        return Err(Error::KeyAlreadyExists);
                    }
                }
                _ => continue,
            }
        }
        Ok(())
    }
}

impl State<ddk::authorization::Authorization, BitName> for BitNamesState {
    const NUM_DBS: u32 = 5;
    type Error = Error;

    fn new(env: &heed::Env) -> Result<Self, Self::Error> {
        let key_to_value = env.create_database(Some("key_to_value"))?;
        Ok(Self { key_to_value })
    }

    fn validate_filled_transaction(
        &self,
        txn: &heed::RoTxn,
        _height: u32,
        _state: &ddk::state::State<ddk::authorization::Authorization, BitName>,
        transaction: &ddk::types::FilledTransaction<BitName>,
    ) -> Result<(), Self::Error> {
        self.validate_keys_unique(txn, &transaction.transaction)?;
        Ok(())
    }

    fn validate_body(
        &self,
        txn: &heed::RoTxn,
        _height: u32,
        _state: &ddk::state::State<ddk::authorization::Authorization, BitName>,
        body: &ddk::types::Body<ddk::authorization::Authorization, BitName>,
    ) -> Result<(), Self::Error> {
        for transaction in &body.transactions {
            self.validate_keys_unique(txn, transaction)?;
        }
        Ok(())
    }

    fn connect_body(
        &self,
        txn: &mut heed::RwTxn,
        _height: u32,
        _state: &ddk::state::State<ddk::authorization::Authorization, BitName>,
        body: &ddk::types::Body<ddk::authorization::Authorization, BitName>,
    ) -> Result<(), Self::Error> {
        for transaction in &body.transactions {
            for output in &transaction.outputs {
                match output.content {
                    ddk::types::Content::Custom(BitName::KeyValue { key, value }) => {
                        self.key_to_value.put(txn, &key, &value)?;
                    }
                    _ => continue,
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("heed error")]
    Heed(#[from] heed::Error),
    #[error("key already exists")]
    KeyAlreadyExists,
}

impl ddk::node::CustomError for Error {}
