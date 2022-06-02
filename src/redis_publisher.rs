
use redis::AsyncCommands;

pub async fn publish(channel: String, message: crate::message::Message, redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>) -> redis::RedisResult<()> {
    let client = redis.lock().unwrap().client.clone();
    let mut publish_connection = client.get_async_connection().await?;
    publish_connection.publish(channel, serde_json::to_string(&message).expect("Failed to parse message")).await?;
    Ok(())
}