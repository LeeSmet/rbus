pub mod error;

use error::ZbusError;

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use rmp_serde::{decode as mpDecode, encode as mpEncode};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use async_trait::async_trait;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectID {
    name: String,
    version: String,
}

impl ObjectID {
    pub fn new(name: &str, version: &str) -> ObjectID {
        ObjectID {
            name: name.into(),
            version: version.into(),
        }
    }
}

impl ToString for ObjectID {
    fn to_string(&self) -> String {
        let mut s = self.name.clone();
        if self.version != "" {
            s += &format!("@{}", self.version);
        }

        s
    }
}

impl From<ObjectID> for String {
    fn from(o: ObjectID) -> Self {
        o.to_string()
    }
}

impl TryFrom<String> for ObjectID {
    type Error = ZbusError;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        let mut parts = s.split('@');

        // get name
        let name = match parts.next() {
            Some(n) => n,
            None => return Err(ZbusError::BadObjectID),
        };

        // get a possible version, default to "" if nothing is left
        let version = parts.next().unwrap_or_default();

        // since there should only be 2 parts, the iterator must be empty here
        if parts.next().is_some() {
            return Err(ZbusError::BadObjectID);
        }

        Ok(ObjectID::new(name, version))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(from = "String")]
#[serde(into = "String")]
pub struct Id(String);

impl Id {
    fn new() -> Id {
        Id(uuid::Uuid::new_v4().to_string())
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(s)
    }
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        id.0
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Request {
    #[serde(rename = "ID")]
    id: Id,
    arguments: Vec<ByteBuf>,
    object: ObjectID,
    reply_to: Id,
    method: String,
}

impl Request {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn args(&self) -> &[ByteBuf] {
        &self.arguments
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Response {
    #[serde(rename = "ID")]
    pub id: Id,
    pub arguments: Vec<ByteBuf>,
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RemoteError {
    pub message: String,
}

#[async_trait]
pub trait Handler {
    async fn dispatch(&self, request: &Request) -> Result<Response, error::ZbusError>;
}

pub struct Server<'a> {
    module: String,
    handlers: HashMap<ObjectID, &'a dyn Handler>,
    con: redis_async::client::paired::PairedConnection,
    workers: usize,
}

impl<'a> Server<'a> {
    pub async fn new(
        addr: &std::net::SocketAddr,
        module: String,
        workers: usize,
    ) -> Result<Server<'_>, Box<dyn std::error::Error>> {
        Ok(Server {
            con: redis_async::client::paired_connect(addr).await?,
            handlers: HashMap::new(),
            module,
            workers,
        })
    }

    pub async fn run(&self) {
        loop {
            log::debug!("Loop iteration");

            let keys: Vec<String> = self
                .handlers
                .keys()
                .map(|k| format!("{}.{}", self.module, k.to_string()))
                .collect();
            // FIXME: type is (String, Vec<u8>)
            let req: Result<Option<Vec<Vec<u8>>>, _> = self
                .con
                .clone()
                .send(
                    redis_async::resp_array!["BLPOP"]
                        .append(keys)
                        .append(&["10".to_owned()]),
                )
                .await;

            if let Err(ref e) = req {
                log::warn!("Error while trying to pop value: {:?}", e);
                continue;
            }

            // this unwrap is safe as we already checked for a possible error above.
            let req = match req.unwrap() {
                Some(data) => data,
                None => continue,
            };

            // BLPOP returns both the key and a value, so we need 2 results
            if req.len() != 2 {
                log::warn!("Invalid response from server");
            }

            let req: Request = match mpDecode::from_slice(&req[1]) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Failed to decode request: {}", e);
                    continue;
                }
            };

            log::info!("Got a request: {:?}", req);

            // this map access requires the key to be checked first
            let response = match self.handlers[&req.object].dispatch(&req).await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Probably failed to decode args: {}", e);
                    continue;
                }
            };

            log::info!("Got response: {:?}", response);

            let payload = match mpEncode::to_vec(&response) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Failed to encode response: {}", e);
                    continue;
                }
            };

            if let Err(e) = self
                .con
                .send::<u64>(redis_async::resp_array!["RPUSH", req.reply_to.0, payload])
                .await
            {
                log::warn!("Failed to push reply: {}", e);
                continue;
            };

            log::info!("Response sent to queue {}", response.id.0);

            // TODO set expire
        }
    }

    pub async fn register(&mut self, object: ObjectID, handler: &'a dyn Handler) {
        self.handlers.insert(object, handler);
    }
}
