use crate::tencent::error::Error;
use crate::tencent::model::{ApplicationProgress, GetApplyProcessResponse};
use reqwest::cookie::Jar;
use reqwest::header::ACCEPT;
use reqwest::Url;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const JOIN_QQ: &str = "https://join.qq.com";

pub type ClientResult<T> = Result<T, Error>;

pub struct Client {
    client: reqwest::Client,
    jar: Arc<Jar>,
}

impl Client {
    pub fn new() -> Self {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .build()
            .expect("Failed to build HTTP client");

        Self {
            jar: jar.clone(),
            client,
        }
    }

    pub fn with_token(token: &String) -> Self {
        let instance = Self::new();
        instance.update_token(token);
        instance
    }

    pub fn update_token(&self, value: &String) {
        let url = JOIN_QQ.parse::<Url>().expect("Tencent becomes no URL");
        self.jar
            .add_cookie_str(format!("UserInfo={}", value).as_str(), &url);
    }

    pub async fn get_application_progress(&self) -> ClientResult<ApplicationProgress> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards.");
        let url = format!(
            "{}/api/v1/apply/getApplyProcess?timestamp={}",
            JOIN_QQ,
            now.as_millis()
        );
        let res = self
            .client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| Error::Http(e))?;
        if res.status().is_success() {
            Ok(res
                .json::<GetApplyProcessResponse>()
                .await
                .map_err(|e| Error::Parse(e))?)
                .map(|r| r.data)
        } else if res.status().is_client_error() {
            Err(Error::TokenExpired)
        } else {
            Err(Error::Http(res.error_for_status().err().unwrap()))
        }
    }
}
