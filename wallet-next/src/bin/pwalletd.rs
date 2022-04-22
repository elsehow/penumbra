#[allow(clippy::clone_on_copy)]
use anyhow::{Context, Result};
use penumbra_proto::client::oblivious::{
    oblivious_query_client::ObliviousQueryClient, AssetListRequest, ChainParamsRequest,
    CompactBlockRangeRequest,
};
use sqlx::sqlite::SqlitePool;
use std::env;
use tonic::transport::{Channel, Server};
use tracing::instrument;

use penumbra_wallet_next::Storage;

use std::path::PathBuf;

use directories::ProjectDirs;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pwalletd",
    about = "The Penumbra wallet daemon.",
    version = env!("VERGEN_GIT_SEMVER"),
)]
struct Opt {
    /// Command to run.
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Start running the wallet daemon.
    Start {
        /// The path used to store the SQLite state database.
        #[structopt(short, long)]
        sqlite_path: PathBuf,
        /// Bind the services to this host.
        #[structopt(short, long, default_value = "127.0.0.1")]
        host: String,
        /// Bind the wallet gRPC server to this port.
        #[structopt(short, long, default_value = "26668")]
        grpc_port: u16,
        /// The address of the pd+tendermint node.
        #[structopt(short, long, default_value = "testnet.penumbra.zone")]
        node: String,
        /// The port to use to speak to pd's light wallet server.
        #[structopt(short, long, default_value = "26666")]
        oblivious_query_port: u16,
        /// The port to use to speak to pd's thin wallet server.
        #[structopt(short, long, default_value = "26667")]
        specific_query_port: u16,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let opt = Opt::from_args();

    match opt.cmd {
        Command::Start {
            sqlite_path,
            host,
            grpc_port,
            node,
            oblivious_query_port,
            specific_query_port,
        } => {
            tracing::info!(
                ?sqlite_path,
                ?host,
                ?grpc_port,
                ?node,
                ?oblivious_query_port,
                ?specific_query_port,
                "starting pwalletd"
            );

            let mut client =
                ObliviousQueryClient::connect(format!("http://{}:{}", node, oblivious_query_port))
                    .await
                    .context("Unable to connect to oblivious query server")?;

            let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
            let storage = Storage::new(pool);
            storage.migrate().await?;

            // TODO: select a custody service here to provide the wallet data source and update local sqlite storage as needed

            // Fetch chain params if necessary so we can .expect() on them.
            if storage.chain_params().await.is_none() {
                chain_params(&client, &mut storage).await?;
            }

            // Always sync pwalletd on startup.
            sync(&client, &mut storage).await?;
            // Retrieve asset list
            assets(&client, &mut storage).await?;

            // TODO: this is just a sqlite usage stub

            let row = storage.insert_table().await?;
            let x = storage.read_table().await?;

            println!(
                "Hello, pwalletd! I got stuff from sqlite: row {} value {}",
                row, x
            );

            // TODO: start gRPC service and respond to command requests
            // let wallet_server = tokio::spawn(
            //     Server::builder()
            //         .trace_fn(|req| match remote_addr(req) {
            //             Some(remote_addr) => tracing::error_span!("wallet_query", ?remote_addr),
            //             None => tracing::error_span!("wallet_query"),
            //         })
            //         .add_service(WalletServer::new(storage.clone()))
            //         .serve(
            //             format!("{}:{}", host, wallet_query_port)
            //                 .parse()
            //                 .expect("this is a valid address"),
            //         ),
            // );
        }
    }

    Ok(())
}

pub async fn sync(client: &ObliviousQueryClient<Channel>, storage: &mut Storage) -> Result<()> {
    tracing::info!("starting client sync");

    let start_height = storage
        .last_block_height()
        .await
        .map(|h| h + 1)
        .unwrap_or(0);
    let mut stream = client
        .compact_block_range(tonic::Request::new(CompactBlockRangeRequest {
            start_height,
            end_height: 0,
            chain_id: storage
                .chain_id()
                .await
                .ok_or_else(|| anyhow::anyhow!("missing chain_id"))?,
        }))
        .await?
        .into_inner();

    let mut count = 0;
    while let Some(block) = stream.message().await? {
        storage.scan_block(block.try_into()?)?;
        // very basic form of intermediate checkpointing
        count += 1;
        if count % 1000 == 1 {
            storage.commit()?;
            tracing::info!(height = ?storage.last_block_height().await.unwrap(), "syncing...");
        }
    }

    storage.prune_timeouts();
    storage.commit()?;
    tracing::info!(end_height = ?storage.last_block_height().await.unwrap(), "finished sync");
    Ok(())
}

pub async fn assets(client: &ObliviousQueryClient<Channel>, storage: &mut Storage) -> Result<()> {
    // Update asset registry.
    let request = tonic::Request::new(AssetListRequest {
        chain_id: storage.chain_id().await.unwrap_or_default(),
    });
    let assets: KnownAssets = client.asset_list(request).await?.into_inner().try_into()?;
    for asset in assets.0 {
        storage
            .asset_cache_mut()
            .extend(std::iter::once(asset.denom));
    }

    storage.commit()?;
    tracing::info!("updated asset registry");
    Ok(())
}

/// Fetches the global chain parameters and stores them on `storage`.

pub async fn chain_params(
    client: &ObliviousQueryClient<Channel>,
    storage: &mut Storage,
) -> Result<()> {
    let params = client
        .chain_params(tonic::Request::new(ChainParamsRequest {
            chain_id: storage.chain_id().await.unwrap_or_default(),
        }))
        .await?
        .into_inner()
        .into();

    tracing::info!(?params, "saving chain params");

    *storage.chain_params_mut() = Some(params);
    storage.commit()?;
    Ok(())
}
