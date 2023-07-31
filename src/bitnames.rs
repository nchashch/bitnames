use ddk::heed;
use ddk::node::State;
use ddk::types::{GetValue, Hash, Transaction};
use heed::{types::*, Database};
use serde::{Deserialize, Serialize};

// Custom sidechain specific output type. It must derive all of these traits.
//
// A sidechain Output has type
//
// struct Output<C> {
// address: Address,
// content: Content<C>,
// }
//
// Content is:
//
// enum Content<C> {
// Value(u64), // regular value output, used for deposits.
// Withdrawal { value: u64, main_address: bitcoin::Address, main_fee: u64 },
// Custom(C), // custom sidechain specific output content.
// }
//
// BitName is a concrete implementation for the C type parameter.
//
// see ddk/src/types/types.rs for actual definitions of these types.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BitName {
    KeyValue { key: Hash, value: Hash },
}

// Custom output type must implement GetValue, which should return the value of this output in
// sats.
impl GetValue for BitName {
    fn get_value(&self) -> u64 {
        // Because key value pairs should be accounted for in the value_in > value_out calculation,
        // they have 0 value in sats as far as the protocol is concerned.
        0
    }
}

// Sustom sidechain specific state. It must derive clone, it should only contain heed databases.
#[derive(Clone)]
pub struct BitNamesState {
    // heed also let's you use arbitrary types implementing serde::Serialize and serde::Deserialize
    // as keys and values, like:
    //
    // Database<SerdeBincode<MyTypeA>, SerdeBincodee<MyTypeB>>
    //
    // Internally in ddk everything is encoded with bincode -- both network representation and db
    // representation.
    //
    // Since Hash is just a [u8; 32] we don't need to serialize it, since it is already a series of
    // bytes.
    key_to_value: Database<OwnedType<Hash>, OwnedType<Hash>>,
}

impl BitNamesState {
    // Convenience method to avoid repeating the same code twice later.
    fn validate_keys_unique(
        &self,
        // txn is a heed db transaction that lets us do db operations atomically.
        //
        // txn is created by ddk and then passed to all of the necessary methods, then if there are
        // no errors, ddk calls txn.commit()?;
        txn: &heed::RoTxn,
        // Transaction is a sidechain transaction. It is generic over the custom output type, that
        // is why we must pass in the BitName type parameter.
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

    // Boilerplate method to create all heed databases.
    fn new(env: &heed::Env) -> Result<Self, Self::Error> {
        let key_to_value = env.create_database(Some("key_to_value"))?;
        Ok(Self { key_to_value })
    }

    // Validate an individual transaction.
    fn validate_filled_transaction(
        &self,
        txn: &heed::RoTxn,
        _height: u32,
        _state: &ddk::state::State<ddk::authorization::Authorization, BitName>,
        // A FilledTransaction includes actual output data for spent utxos:
        //
        // FilledTransaction {
        // spent_utxos: Vec<Output>, // Output includes value, address, etc.
        // transaction: Transaction,
        // }
        //
        // Transaction {
        // inputs: Vec<OutPoint>, // OutPoint only includes txid, vout (or equivalent for deposits and coinbases).
        // outputs: Vec<Output>,
        // }
        //
        // There is also AuthorizedTransaction, which includes Authorization s, i.e. witness data.
        // Though it is not dealt with here.
        //
        // see ddk/src/types/types.rs for actual definitions of these types.
        transaction: &ddk::types::FilledTransaction<BitName>,
    ) -> Result<(), Self::Error> {
        self.validate_keys_unique(txn, &transaction.transaction)?;
        Ok(())
    }

    // Validate a block body. A body is just a block without a header. Headers are handled
    // internally by ddk.
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

    // Assuming the body is valid, execute the state transition for this block body.
    //
    // This method is *sort of* equivalent to bitcoin's ConnectBlock method.
    //
    // Things like utxos, withdrawals, and deposits are handled internally by ddk.
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
                        // In practice this means just updating all of the heed dbs according to
                        // consensus rules.
                        self.key_to_value.put(txn, &key, &value)?;
                    }
                    _ => continue,
                }
            }
        }
        Ok(())
    }
}

// The custom error type defining all the possible ways things can go wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("heed error")]
    Heed(#[from] heed::Error),
    #[error("key already exists")]
    KeyAlreadyExists,
}

// This is just a hack to make the type checker happy.
impl ddk::node::CustomError for Error {}
