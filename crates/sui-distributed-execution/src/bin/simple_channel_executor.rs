use clap::*;
use std::path::PathBuf;
use std::sync::Arc;
use sui_config::{Config, NodeConfig};
use sui_distributed_execution::seqn_worker;
use sui_distributed_execution::exec_worker;
use sui_types::multiaddr::Multiaddr;
use tokio::sync::mpsc;

const GIT_REVISION: &str = {
    if let Some(revision) = option_env!("GIT_REVISION") {
        revision
    } else {
        let version = git_version::git_version!(
            args = ["--always", "--dirty", "--exclude", "*"],
            fallback = ""
        );

        if version.is_empty() {
            panic!("unable to query git revision");
        }
        version
    }
};
const VERSION: &str = const_str::concat!(env!("CARGO_PKG_VERSION"), "-", GIT_REVISION);

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
#[clap(name = env!("CARGO_BIN_NAME"))]
#[clap(version = VERSION)]
struct Args {
    #[clap(long)]
    pub config_path: PathBuf,

    /// Specifies the watermark up to which I will download checkpoints
    #[clap(long)]
    download: Option<u64>,

    /// Specifies the watermark up to which I will execute checkpoints
    #[clap(long)]
    execute: Option<u64>,

    #[clap(long, help = "Specify address to listen on")]
    listen_address: Option<Multiaddr>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = NodeConfig::load(&args.config_path).unwrap();
    let genesis = Arc::new(config.genesis().expect("Could not load genesis"));
    let mut sw_state = seqn_worker::SequenceWorkerState::new(&config).await;
    let metrics = sw_state.metrics.clone();
    let mut ew_state = exec_worker::ExecutionWorkerState::new();
    ew_state.init_store(&genesis);

    let (epoch_start_sender, epoch_start_receiver) = mpsc::channel(32);
    let (tx_sender, tx_receiver) = mpsc::channel(1000);
    let (epoch_end_sender, epoch_end_receiver) = mpsc::channel(32);

    // Run Sequence Worker
    let sw_handler = tokio::spawn(async move {
        sw_state.run(
            config.clone(), 
            args.download, 
            args.execute,
            epoch_start_sender, 
            tx_sender, 
            epoch_end_receiver
        ).await;
    });

    // Run Execution Worker
    let mut ew_handler_opt = None;
    if let Some(watermark) = args.execute {
        ew_handler_opt = Some(tokio::spawn(async move {
            ew_state.run(
                metrics,
                watermark,
                epoch_start_receiver,
                tx_receiver,
                epoch_end_sender,
            ).await;
        }));
    }

    // Wait for workers to terminate
    sw_handler.await.expect("sw failed");
    if let Some(ew_handler) = ew_handler_opt {
        ew_handler.await.expect("ew failed");
    }
}
