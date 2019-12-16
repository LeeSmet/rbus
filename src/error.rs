use rmp_serde::decode::Error as rmpderror;
use rmp_serde::encode::Error as rmpeerror;

#[derive(Debug)]
pub enum ZbusError {
    BadObjectID,
    DecodeError,
    EncodeError,
    E(String),
}

impl std::fmt::Display for ZbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ZbusError::BadObjectID => write!(f, "Could not parse object ID"),
            ZbusError::DecodeError => write!(f, "Decoding failed"),
            ZbusError::EncodeError => write!(f, "Encoding failed"),
            ZbusError::E(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for ZbusError {}

impl From<rmpderror> for ZbusError {
    fn from(_: rmpderror) -> ZbusError {
        ZbusError::DecodeError
    }
}

impl From<rmpeerror> for ZbusError {
    fn from(_: rmpeerror) -> ZbusError {
        ZbusError::EncodeError
    }
}

impl From<String> for ZbusError {
    fn from(s: String) -> ZbusError {
        ZbusError::E(s)
    }
}
