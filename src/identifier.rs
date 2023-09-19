use std::{convert::Infallible, fmt::Display, num::NonZeroU8, str::FromStr};

use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct TrackIdentifier {
    pub album_id: Uuid,
    pub disc_id: NonZeroU8,
    pub track_id: NonZeroU8,
}

impl FromStr for TrackIdentifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sp = s.split('/');

        let album_id = sp.next().unwrap().parse().unwrap();
        let disc_id = sp.next().unwrap().parse().unwrap();
        let track_id = sp.next().unwrap().parse().unwrap();

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
