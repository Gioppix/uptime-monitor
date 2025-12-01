use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{EnumIter, IntoEnumIterator};
use utoipa::ToSchema;

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    Serialize,
    Deserialize,
    ToSchema,
    Hash,
)]
pub enum Region {
    Fsn1, // Falkenstein, Germany
    Hel1, // Helsinki, Finland
    Nbg1, // Nuremberg, Germany
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
            Region::Fsn1 => "fsn1",
            Region::Hel1 => "hel1",
            Region::Nbg1 => "nbg1",
        }
    }

    pub fn from_identifier(identifier: &str) -> anyhow::Result<Self> {
        match identifier {
            "fsn1" => Ok(Region::Fsn1),
            "hel1" => Ok(Region::Hel1),
            "nbg1" => Ok(Region::Nbg1),
            _ => Err(anyhow!("unknown region identifier: {identifier}")),
        }
    }

    pub fn get_all_region_identifiers() -> Vec<&'static str> {
        Region::iter().map(|r| r.to_identifier()).collect()
    }
}
