use std::{
    fmt::Display,
    num::{NonZeroU8, ParseIntError},
    str::FromStr,
};

use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct TrackIdentifier {
    pub album_id: Uuid,
    pub disc_id: NonZeroU8,
    pub track_id: NonZeroU8,
}

#[derive(Debug, Clone, Copy)]
pub struct ParseError;

impl From<uuid::Error> for ParseError {
    fn from(_: uuid::Error) -> Self {
        Self
    }
}

impl From<ParseIntError> for ParseError {
    fn from(_: ParseIntError) -> Self {
        Self
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fail to parse track identifier")
    }
}

impl std::error::Error for ParseError {}

impl FromStr for TrackIdentifier {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sp = s.split('/');

        let album_id = sp.next().ok_or(ParseError)?.parse()?;
        let disc_id = sp.next().ok_or(ParseError)?.parse()?;
        let track_id = sp.next().ok_or(ParseError)?.parse()?;

        Ok(Self {
            album_id,
            disc_id,
            track_id,
        })
    }
}

impl Display for TrackIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.album_id, self.disc_id, self.track_id)
    }
}
