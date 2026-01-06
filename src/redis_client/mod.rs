use anyhow::Result;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};

pub struct RedisClient {
    connection: MultiplexedConnection,
}

#[derive(Debug, Clone)]
pub enum RedisValue {
    String(String),
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(Vec<(String, String)>),
    Stream(String), // Simplified for now
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisType {
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    Unknown,
}

impl RedisClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let client = Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self { connection })
    }

    pub async fn scan_keys(&mut self, pattern: &str, count: usize) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (new_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(count)
                .query_async(&mut self.connection)
                .await?;

            keys.extend(batch);
            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        Ok(keys)
    }

    pub async fn get_type(&mut self, key: &str) -> Result<RedisType> {
        let type_str: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut self.connection)
            .await?;

        Ok(match type_str.as_str() {
            "string" => RedisType::String,
            "list" => RedisType::List,
            "set" => RedisType::Set,
            "zset" => RedisType::ZSet,
            "hash" => RedisType::Hash,
            "stream" => RedisType::Stream,
            _ => RedisType::Unknown,
        })
    }

    pub async fn get_value(&mut self, key: &str) -> Result<RedisValue> {
        let key_type = self.get_type(key).await?;

        match key_type {
            RedisType::String => {
                let val: String = self.connection.get(key).await?;
                Ok(RedisValue::String(val))
            }
            RedisType::List => {
                let val: Vec<String> = self.connection.lrange(key, 0, -1).await?;
                Ok(RedisValue::List(val))
            }
            RedisType::Set => {
                let val: Vec<String> = self.connection.smembers(key).await?;
                Ok(RedisValue::Set(val))
            }
            RedisType::ZSet => {
                let val: Vec<(String, f64)> = self.connection.zrange_withscores(key, 0, -1).await?;
                Ok(RedisValue::ZSet(val))
            }
            RedisType::Hash => {
                let val: Vec<(String, String)> = self.connection.hgetall(key).await?;
                Ok(RedisValue::Hash(val))
            }
            _ => Ok(RedisValue::None),
        }
    }

    pub async fn get_ttl(&mut self, key: &str) -> Result<i64> {
        let ttl: i64 = self.connection.ttl(key).await?;
        Ok(ttl)
    }

    pub async fn set_string(&mut self, key: &str, value: &str) -> Result<()> {
        let _: () = self.connection.set(key, value).await?;
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        let _: () = self.connection.del(key).await?;
        Ok(())
    }
}
