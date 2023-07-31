use bitnames::{BitName, BitNamesState};
use ddk::authorization::Authorization;
use std::net::SocketAddr;

mod bitnames;

type Node = ddk::node::Node<Authorization, BitName, BitNamesState>;
type Wallet = ddk::wallet::Wallet<BitName>;
type Miner = ddk::miner::Miner<Authorization, BitName>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_NET_PORT: u16 = 4000;
    let net_port = DEFAULT_NET_PORT;
    let net_addr: SocketAddr = format!("127.0.0.1:{net_port}").parse()?;
    let datadir = project_root::get_project_root()?.join("target/bitnames");
    let wallet_path = datadir.join("wallet.mdb");
    let _node = Node::new(&datadir, net_addr, "localhost", 18443)?;
    let _wallet = Wallet::new(&wallet_path)?;
    let _miner = Miner::new(0, "localhost", 18443)?;
    Ok(())
}
