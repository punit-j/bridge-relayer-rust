pub const NEAR_EVENTS_TRACER_TARGET: &str = "near_events_tracker";
pub const EVENT_PROCESSOR_TARGET: &str = "event_processor";
pub const PENDING_TRANSACTION_TARGET: &str = "pending_transactions";

pub fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    let _result = tracing::subscriber::set_global_default(subscriber);
}
