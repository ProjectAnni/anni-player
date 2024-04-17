use reqwest::blocking::{Client, Response};

use crate::identifier::TrackIdentifier;

pub struct ProviderProxy {
    url: String,
    client: Client,
    auth: String,
}

impl ProviderProxy {
    pub fn new(url: String, auth: String, client: Client) -> Self {
        Self { url, auth, client }
    }

    pub fn format_url(&self, track: TrackIdentifier) -> String {
        format!("{}/{}?auth={}&quality=lossless", self.url, track, self.auth)
    }

    pub fn get(&self, track: TrackIdentifier) -> reqwest::Result<Response> {
        self.client
            .get(self.format_url(track))
            // .header("Authorization", &self.auth)
            .send()
    }

    pub fn head(&self, track: TrackIdentifier) -> reqwest::Result<Response> {
        self.client
            .get(self.format_url(track))
            // .header("Authorization", &self.auth)
            .send()
    }
}
