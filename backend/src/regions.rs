use anyhow::anyhow;
use std::str::FromStr;

#[derive(Copy, Clone)]
pub enum Region {
    UsWest,
    UsEast,
    EuWest,
    SoutheastAsia,
}

impl FromStr for Region {
    type Err = anyhow::Error;

    fn from_str(identifier: &str) -> Result<Self, Self::Err> {
        Self::from_identifier(identifier)
    }
}

impl Region {
    pub fn to_identifier(self) -> &'static str {
        match self {
            Region::UsWest => "us-west2",
            Region::UsEast => "us-east4-eqdc4a",
            Region::EuWest => "europe-west4-drams3a",
            Region::SoutheastAsia => "asia-southeast1-eqsg3a",
        }
    }

    pub fn from_identifier(identifier: &str) -> anyhow::Result<Self> {
        match identifier {
            "us-west2" => Ok(Region::UsWest),
            "us-east4-eqdc4a" => Ok(Region::UsEast),
            "europe-west4-drams3a" => Ok(Region::EuWest),
            "asia-southeast1-eqsg3a" => Ok(Region::SoutheastAsia),
            _ => Err(anyhow!("unknown region identifier: {identifier}")),
        }
    }
}
