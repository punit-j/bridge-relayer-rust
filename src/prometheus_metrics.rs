use prometheus::core::{AtomicU64, GenericGauge};
use prometheus::Registry;

use lazy_static::lazy_static;
use warp::Filter;
use warp::Rejection;
use warp::Reply;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref INIT_TRANSFERS_COUNT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "init_transfers_count",
        "The total number of detected initialized token transfers"
    )
    .expect("metric can't be created");

    pub static ref NEAR_LAST_PROCESSED_BLOCK_HEIGHT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "near_last_processed_block_height",
        "The height of the near block processed in near events tracer"
    )
    .expect("metric can't be created");


    pub static ref PENDING_TRANSACTIONS_COUNT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "pending_transactions_count",
        "The total number of of submitted transactions to Ethereum"
    )
    .expect("metric can't be created");

    pub static ref SUCCESS_TRANSACTIONS_COUNT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "success_transactions_count",
        "The total number of successful transactions to Ethereum"
    )
    .expect("metric can't be created");

    pub static ref UNLOCKED_TRANSACTIONS_COUNT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "unlocked_transactions_count",
        "The total number of unlocked transactions"
    )
    .expect("metric can't be created");

    pub static ref LAST_ETH_BLOCK_ON_NEAR: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "last_eth_block_on_near",
        "The last ethereum block on light client on Near"
    )
    .expect("metric can't be created");

    pub static ref FAIL_TRANSACTIONS_COUNT:  GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "fail_transactions_count",
        "The total number of fail transactions to Ethereum"
    )
    .expect("metric can't be created");

    pub static ref SKIP_TRANSACTIONS_COUNT:  GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "skip_transactions_count",
        "The total number of skipped transactions (relayer decided don't process these transactions)"
    )
    .expect("metric can't be created");

    pub static ref CONNECTION_ERRORS: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "connection_errors",
        "The total number of connection error"
    )
    .expect("metric can't be created");

    pub static ref BALANCE_ERRORS: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "balance_errors",
        "The total number of balance error"
    )
    .expect("metric can't be created");

    pub static ref UNLOCK_TOKENS_CURRENT_NEAR_BLOCK_HEIGHT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "unlock_tokens_current_near_block_height",
        "The current block height on near in unlock tokens worker"
    )
    .expect("metric can't be created");

    pub static ref PENDING_TRANSACTIONS_CURRENT_ETH_BLOCK_HEIGHT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "pending_transactions_current_eth_block_height",
        "The current block height on eth in pending transactions worker"
    )
    .expect("metric can't be created");

    pub static ref NEAR_EVENTS_PROCESSOR_CURRENT_ETH_BLOCK_HEIGHT: GenericGauge<AtomicU64> = GenericGauge::<AtomicU64>::new(
        "near_events_processor_current_eth_block_height",
        "The current block height on eth in near events processor worker"
    )
    .expect("metric can't be created");
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(INIT_TRANSFERS_COUNT.clone()))
        .expect("init_transfers_count can't be registered");

    REGISTRY
        .register(Box::new(NEAR_LAST_PROCESSED_BLOCK_HEIGHT.clone()))
        .expect("near_last_processed_block_height can't be registered");

    REGISTRY
        .register(Box::new(PENDING_TRANSACTIONS_COUNT.clone()))
        .expect("pending_transactions_count can't be registered");

    REGISTRY
        .register(Box::new(SUCCESS_TRANSACTIONS_COUNT.clone()))
        .expect("success_transactions_count can't be registered");

    REGISTRY
        .register(Box::new(UNLOCKED_TRANSACTIONS_COUNT.clone()))
        .expect("unlocked_transactions_count can't be registered");

    REGISTRY
        .register(Box::new(LAST_ETH_BLOCK_ON_NEAR.clone()))
        .expect("last_eth_block_on_near can't be registered");

    REGISTRY
        .register(Box::new(FAIL_TRANSACTIONS_COUNT.clone()))
        .expect("fail_transactions_count can't be registered");

    REGISTRY
        .register(Box::new(SKIP_TRANSACTIONS_COUNT.clone()))
        .expect("skip_transactions_count can't be registered");

    REGISTRY
        .register(Box::new(CONNECTION_ERRORS.clone()))
        .expect("connection_errors can't be registered");

    REGISTRY
        .register(Box::new(BALANCE_ERRORS.clone()))
        .expect("balance_errors can't be registered");

    REGISTRY
        .register(Box::new(UNLOCK_TOKENS_CURRENT_NEAR_BLOCK_HEIGHT.clone()))
        .expect("unlock_tokens_current_near_block_height can't be registered");

    REGISTRY
        .register(Box::new(
            PENDING_TRANSACTIONS_CURRENT_ETH_BLOCK_HEIGHT.clone(),
        ))
        .expect("pending_transactions_current_eth_block_height can't be registered");

    REGISTRY
        .register(Box::new(
            NEAR_EVENTS_PROCESSOR_CURRENT_ETH_BLOCK_HEIGHT.clone(),
        ))
        .expect("near_events_processor_current_eth_block_height can't be registered");
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        eprintln!("could not encode custom metrics: {:?}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        eprintln!("could not encode prometheus metrics: {:?}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };

    res.push_str(&res_custom);
    Ok(res)
}

pub fn run_prometheus_service(port: u16) {
    register_custom_metrics();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);

    let rt = tokio_02::runtime::Runtime::new()
        .expect("Error on creating runtime for Prometheus service");
    let handle = rt.handle();

    tracing::info!("Started Prometheus on port {}", port);
    handle.block_on(warp::serve(metrics_route).run(([0, 0, 0, 0], port)));
}
