//! map2 command decoding.

use crate::server::ParseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Map2Item {}

#[derive(Debug, Clone, PartialEq)]
pub enum TileOp {}

pub fn parse_map2(_data: &[u8]) -> Result<Vec<Map2Item>, ParseError> {
    Ok(vec![])
}
