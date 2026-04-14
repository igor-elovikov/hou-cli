use super::{Client, Platform, Product, Release, Status};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum BuildSpec {
    Number(u32),
    Production,
}

impl BuildSpec {
    fn as_json(&self) -> Value {
        match self {
            BuildSpec::Number(n) => Value::String(n.to_string()),
            BuildSpec::Production => Value::String("production".into()),
        }
    }
}

#[derive(Debug)]
pub struct BuildDownload {
    pub date: NaiveDate,
    pub download_url: String,
    pub filename: String,
    pub hash: String,
    pub release: Release,
    pub status: Status,
    pub size: u64,
}

impl<'de> Deserialize<'de> for BuildDownload {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            date: String,
            download_url: String,
            filename: String,
            hash: String,
            releases_list: String,
            status: String,
            size: u64,
        }
        use serde::de::Error;
        let r = Raw::deserialize(d)?;
        let date = NaiveDate::parse_from_str(&r.date, "%Y/%m/%d").map_err(D::Error::custom)?;
        let release = r.releases_list.parse::<Release>().map_err(D::Error::custom)?;
        let status = r.status.parse::<Status>().map_err(D::Error::custom)?;
        Ok(BuildDownload {
            date,
            download_url: r.download_url,
            filename: r.filename,
            hash: r.hash,
            release,
            status,
            size: r.size,
        })
    }
}

pub struct BuildDownloadQuery<'a> {
    client: &'a Client,
    product: Product,
    version: String,
    build: BuildSpec,
    platform: Option<Platform>,
}

impl<'a> BuildDownloadQuery<'a> {
    pub(super) fn new(
        client: &'a Client,
        product: Product,
        version: String,
        build: BuildSpec,
    ) -> Self {
        Self { client, product, version, build, platform: None }
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn send(self) -> Result<BuildDownload> {
        let mut kwargs = serde_json::Map::new();
        kwargs.insert("product".into(), self.product.as_api_str().into());
        kwargs.insert("version".into(), self.version.into());
        kwargs.insert("build".into(), self.build.as_json());
        if let Some(p) = self.platform {
            kwargs.insert("platform".into(), p.as_api_str().into());
        }

        let result = self.client.call(
            "download.get_daily_build_download",
            Value::Array(vec![]),
            Value::Object(kwargs),
        )?;

        serde_json::from_value(result).context("failed to deserialize daily build download")
    }
}
