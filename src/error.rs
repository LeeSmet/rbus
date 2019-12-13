#[derive(Debug)]
pub enum ZbusError {
    BadObjectID,
    E(String),
}

impl std::fmt::Display for ZbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ZbusError::BadObjectID => write!(f, "Could not parse object ID"),
            ZbusError::E(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for ZbusError {}
