use reqwest::blocking::{Client, Response};

use crate::identifier::TrackIdentifier;

pub struct ProviderProxy {
    url: String,
    client: Client,
    auth: String,
}

impl ProviderProxy {
    pub fn new(url: String, auth: String) -> Self {
        Self {
            url,
            auth,
            client: Client::new(),
        }
    }

    pub fn format_url(&self, track: TrackIdentifier) -> String {
        format!("{}/{}?auth={}", self.url, track, self.auth)
    }

    pub fn get(&self, track: TrackIdentifier) -> reqwest::Result<Response> {
        self.client
            .get(format!("{}/{}?auth={}", self.url, track, self.auth))
            // .header("Authorization", &self.auth)
            .send()
    }

    pub fn head(&self, track: TrackIdentifier) -> reqwest::Result<Response> {
        self.client
            .get(format!("{}/{}?auth={}", self.url, track, self.auth))
            // .header("Authorization", &self.auth)
            .send()
    }
}
