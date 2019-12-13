use async_trait::async_trait;
use rbus::{Handler, Request, Response};
use rmp_serde::{decode, encode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let name = "server".to_owned();
    let id = rbus::ObjectID::new("calculator", "1.0");

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6379);
    let mut server = rbus::Server::new(&socket, name, 1).await.unwrap();
    let calc = Calculator {};
    server.register(id, &calc).await;
    server.run().await;
}

struct Calculator {}

impl Calculator {
    fn add(&self, nums: Vec<f64>) -> f64 {
        let mut result = 0.;
        for num in nums {
            result += num;
        }
        result
    }
}

#[async_trait]
impl Handler for Calculator {
    async fn dispatch(&self, req: &Request) -> Result<Response, rbus::error::ZbusError> {
        match req.method() {
            "Add" => {
                let mut args0 = Vec::new();
                for arg in req.args() {
                    match decode::from_slice(arg) {
                        Ok(d) => args0.push(d),
                        // TODO: Proper error
                        Err(e) => {
                            return Err(rbus::error::ZbusError::E(format!(
                                "Could not decode element {}",
                                e
                            )))
                        }
                    }
                }
                // TODO: Proper error handling
                let resp_args = self.add(args0);
                let payload = serde_bytes::ByteBuf::from(encode::to_vec(&resp_args).unwrap());
                Ok(Response {
                    id: req.id().clone(),
                    arguments: vec![payload],
                    error: "".to_owned(),
                })
            }
            // TODO: Proper error
            _ => Err(rbus::error::ZbusError::E("No such method".to_string())),
        }
    }
}
