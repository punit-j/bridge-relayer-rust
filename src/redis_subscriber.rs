use futures_util::StreamExt;

pub async fn subscribe(channel: String, redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>) -> redis::RedisResult<()> {
    tokio::spawn(async move {
        let client = redis.lock().unwrap().client.clone();
        let mut pubsub_connection = client.get_async_connection().await.expect("REDIS: Failed to get connection").into_pubsub();
        pubsub_connection.subscribe(channel.clone()).await.expect("Failed to subscribe to the channel");
        let mut pubsub_stream = pubsub_connection.on_message();
        loop {
            let pubsub_msg: String = pubsub_stream.next().await.unwrap().get_payload().expect("Failed to fetch the message");
            crate::message_handler::handle(channel.clone(), pubsub_msg);
        }
    });
    Ok(())
}