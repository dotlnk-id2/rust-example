// use std::sync::Arc;

use std::{
    collections::HashMap,
    fmt::{self, Display},

};

// use std::net::{IpAddr, SocketAddr};

use serde::Deserialize;
// use compact_str::CompactString;

#[derive(Debug, Default, Deserialize)]
pub struct AppConfig {
    pub http_bind: String,
    pub http_port: u16,
    pub http_enable: bool,
    pub tcp_bind: String,
    pub tcp_port: u16,
    pub tcp_enable: bool,
    pub udp_bind: String,
    pub udp_port: u16,
    pub udp_enable: bool,
    pub bind_key: String,
    pub bind_cer: String,
    pub worker_num: usize,
    pub wait_queue_num: usize,
    pub max_connections: usize,
    pub max_connection_rate: usize,
    pub client_request_timeout: usize,
    pub client_disconnect_timeout: usize,
    pub tls_handshake_timeout: usize,
    pub keep_alive: usize,
    pub shutdown_timeout: usize,
    pub stdout_log: bool,
    pub log_enable: bool,
    pub log_path: String,
    pub optl_enable: bool,
    pub optl_endpoint: String,
    pub pool_opt: PoolOpt,
    pub redis_opt: CacheDB,
    pub database: HashMap<String, Database>,
    pub route_mapping: Vec<RouteMapping>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CacheDB {
    pub cluster_type: String,
    pub redis_addr: String,
    pub redis_port: u16,
    pub redis_pass: String,
    pub redis_max_size: u32,
    pub redis_min_idle: u32,
    pub redis_max_lifetime: u64,
    pub redis_conn_timeout: u64,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoolOpt {
    pub max_conn: u32,
    pub min_conn: u32,
    pub conn_timeout: u64,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub sqlx_log_enable: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Database {
    pub db_alias: String,
    pub db_type: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    pub db_passwd_type: String,
    pub db_passwd: String,
}

impl Database {
    pub fn from_db(&self) -> Result<String, &'static str> {
        match self.db_type.as_str() {
            "mysql" => Ok(format!(
                "mysql://{}:{}@{}:{}/{}",
                self.db_user,
                self.from_raw_pwd()?,
                self.db_host,
                self.db_port,
                self.db_name
            )),
            "pgsql" => Ok(format!(
                "postgres://{}:{}@{}:{}/{}",
                self.db_user,
                self.from_raw_pwd()?,
                self.db_host,
                self.db_port,
                self.db_name
            )),
            "mongdb" => Ok(format!(
                "mongodb+srv://{}:{}@{}:{}/{}",
                self.db_user,
                self.from_raw_pwd()?,
                self.db_host,
                self.db_port,
                self.db_name
            )),
            _ => Err("DB type not found"),
        }
    }

    pub fn from_raw_pwd(&self) -> Result<&str, &'static str> {
        match self.db_passwd_type.as_str() {
            "raw" => Ok(self.db_passwd.as_str()),
            _ => Err("DB type not found"),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RouteMapping {
    pub prefix: String,
    pub target: String,
}
