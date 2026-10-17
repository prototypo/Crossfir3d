//! Crossfire protocol: framing, messages, map2, map view. No I/O.

pub mod client;
pub mod frame;
pub mod map2;
pub mod server;
pub mod view;

pub use client::ClientMsg;
pub use frame::{read_frame, write_frame};
pub use map2::{Map2Item, TileOp, ANIM_MASK, FACE_IS_ANIM};
pub use server::{parse, ParseError, ServerMsg};
pub use view::{Cell, MapView};
