use super::{Client, Platform, Product, Release, Status};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use semver::Version;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug)]
pub struct Build {
    pub build: u32,
    pub version: Version,
    pub product: Product,
    pub platform: Platform,
    pub date: NaiveDate,
    pub release: Release,
    pub status: Status,
}

impl<'de> Deserialize<'de> for Build {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            build: String,
            version: String,
            product: String,
            platform: String,
            date: String,
            release: String,
            status: String,
        }
        use serde::de::Error;
        let r = Raw::deserialize(d)?;
        let build: u32 = r.build.parse().map_err(D::Error::custom)?;
        let version = Version::parse(&format!("{}.{}", r.version, build))
            .map_err(D::Error::custom)?;
        let product = r.product.parse::<Product>().map_err(D::Error::custom)?;
        let platform = r.platform.parse::<Platform>().map_err(D::Error::custom)?;
        let date = NaiveDate::parse_from_str(&r.date, "%Y/%m/%d").map_err(D::Error::custom)?;
        let release = r.release.parse::<Release>().map_err(D::Error::custom)?;
        let status = r.status.parse::<Status>().map_err(D::Error::custom)?;
        Ok(Build { build, version, product, platform, date, release, status })
    }
}

pub struct BuildsQuery<'a> {
    client: &'a Client,
    product: Product,
    version: Option<String>,
    platform: Option<Platform>,
    only_production: Option<bool>,
}

impl<'a> BuildsQuery<'a> {
    pub(super) fn new(client: &'a Client, product: Product) -> Self {
        Self { client, product, version: None, platform: None, only_production: None }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn only_production(mut self) -> Self {
        self.only_production = Some(true);
        self
    }

    pub fn send(self) -> Result<Vec<Build>> {
        let mut kwargs = serde_json::Map::new();
        kwargs.insert("product".into(), self.product.as_api_str().into());
        if let Some(v) = self.version {
            kwargs.insert("version".into(), v.into());
        }
        if let Some(p) = self.platform {
            kwargs.insert("platform".into(), p.as_api_str().into());
        }
        if let Some(op) = self.only_production {
            kwargs.insert("only_production".into(), op.into());
        }

        let result = self.client.call(
            "download.get_daily_builds_list",
            Value::Array(vec![]),
            Value::Object(kwargs),
        )?;

        serde_json::from_value(result).context("failed to deserialize daily builds list")
    }
}
