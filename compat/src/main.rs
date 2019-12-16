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

    fn pow(&self, a: f64, b: f64) -> f64 {
        a.powf(b)
    }

    fn divide(&self, a: f64, b: f64) -> Result<f64, rbus::error::ZbusError> {
        if b == 0. {
            return Err("Can't divide by zero".to_owned().into());
        }
        Ok(a / b)
    }

    fn avg(&self, a: Vec<f64>) -> f64 {
        if a.is_empty() {
            return 0.;
        }
        a.iter().sum::<f64>() / a.len() as f64
    }
}

#[async_trait]
impl Handler for Calculator {
    async fn dispatch(&self, req: &Request) -> Result<Response, rbus::error::ZbusError> {
        match req.method() {
            "Add" => {
                // TODO: min lenght argh check, since variadics are dumb
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
            "Pow" => {
                // arg check
                if req.args().len() != 2 {
                    return Err(rbus::error::ZbusError::E(format!(
                        "Mismatched argument length, expected 2 got {}",
                        req.args().len()
                    )));
                }
                let arg0 = decode::from_slice(&req.args()[0])?;
                let arg1 = decode::from_slice(&req.args()[1])?;
                let resp_args = self.pow(arg0, arg1);
                let payload = serde_bytes::ByteBuf::from(encode::to_vec(&resp_args).unwrap());
                Ok(Response {
                    id: req.id().clone(),
                    arguments: vec![payload],
                    error: "".to_owned(),
                })
            }
            "Divide" => {
                // arg check
                if req.args().len() != 2 {
                    return Err(rbus::error::ZbusError::E(format!(
                        "Mismatched argument length, expected 2 got {}",
                        req.args().len()
                    )));
                }
                let arg0 = decode::from_slice(&req.args()[0])?;
                let arg1 = decode::from_slice(&req.args()[1])?;
                let resp_args = self.divide(arg0, arg1);
                let encoded_args = match resp_args {
                    Ok(r) => vec![
                        serde_bytes::ByteBuf::from(encode::to_vec(&r).unwrap()),
                        serde_bytes::ByteBuf::from(
                            encode::to_vec::<Option<rbus::RemoteError>>(&None).unwrap(),
                        ),
                    ],
                    Err(e) => vec![
                        serde_bytes::ByteBuf::from(encode::to_vec(&f64::default()).unwrap()),
                        serde_bytes::ByteBuf::from(
                            encode::to_vec(&rbus::RemoteError {
                                message: format!("{}", e),
                            })
                            .unwrap(),
                        ),
                    ],
                };
                Ok(Response {
                    id: req.id().clone(),
                    arguments: encoded_args,
                    error: "".to_owned(),
                })
            }
            "Avg" => {
                // arg check
                if req.args().len() != 1 {
                    return Err(rbus::error::ZbusError::E(format!(
                        "Mismatched argument length, expected 1 got {}",
                        req.args().len()
                    )));
                }
                let arg0 = decode::from_slice(&req.args()[0])?;
                let resp_args = self.avg(arg0);
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
