use prometheus::{IntCounter, Registry};

use lazy_static::lazy_static;
use tokio::runtime::Runtime;
use warp::Filter;
use warp::Rejection;
use warp::Reply;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref INIT_TRANSFERS_COUNT: IntCounter = IntCounter::new(
        "init_transfers_count",
        "The total number of detected initialized token transfers"
    )
    .expect("metric can't be created");
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(INIT_TRANSFERS_COUNT.clone()))
        .expect("init_transfers_count can't be registered");
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

    let rt = Runtime::new().expect("Error on creating runtime for Prometheus service");
    let handle = rt.handle();

    println!("Started on port {}", port);
    handle.block_on(warp::serve(metrics_route).run(([0, 0, 0, 0], port)));
}
