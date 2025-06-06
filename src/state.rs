use base64::{Engine, prelude::BASE64_STANDARD};
use colored::Colorize;
use futures::future::join_all;
use regex::RegexBuilder;
use rquest::{
    Client, ClientBuilder, IntoUrl, Method, Proxy, RequestBuilder, Response,
    header::{COOKIE, ORIGIN, REFERER, SET_COOKIE},
    multipart::{Form, Part},
};
use rquest_util::Emulation;
use serde_json::{Value, json};
use tracing::{debug, error, warn};

use std::{collections::HashMap, sync::LazyLock};

use crate::{
    config::{CLEWDR_CONFIG, CookieStatus, ENDPOINT, Reason},
    error::ClewdrError,
    services::cookie_manager::CookieEventSender,
    types::message::ImageSource,
};

/// The client to be used for requests to the Claude.ai
/// This client is used for requests that require a specific emulation
static SUPER_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .emulation(Emulation::Chrome135)
        .build()
        .expect("Failed to create client")
});

/// State of current connection
#[derive(Clone)]
pub struct ClientState {
    pub cookie: Option<CookieStatus>,
    pub event_sender: CookieEventSender,
    pub org_uuid: Option<String>,
    pub conv_uuid: Option<String>,
    cookies: HashMap<String, String>,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub proxy: Option<Proxy>,
}

impl ClientState {
    /// Create a new AppState instance
    pub fn new(event_sender: CookieEventSender) -> Self {
        ClientState {
            event_sender,
            cookie: None,
            org_uuid: None,
            conv_uuid: None,
            cookies: HashMap::new(),
            capabilities: Vec::new(),
            endpoint: CLEWDR_CONFIG.load().endpoint(),
            proxy: CLEWDR_CONFIG.load().rquest_proxy.clone(),
        }
    }

    /// Build a request with the current cookie and proxy settings
    pub fn request(&self, method: Method, url: impl IntoUrl) -> RequestBuilder {
        let r = SUPER_CLIENT
            .request(method, url)
            .header_append(ORIGIN, ENDPOINT)
            .header_append(COOKIE, self.header_cookie());
        let r = if let Some(uuid) = self.conv_uuid.clone() {
            r.header_append(REFERER, format!("{}/chat/{}", ENDPOINT, uuid))
        } else {
            r.header_append(REFERER, format!("{}/chat/new", ENDPOINT))
        };
        if let Some(proxy) = self.proxy.clone() {
            r.proxy(proxy)
        } else {
            r
        }
    }

    /// Checks if the current user has pro capabilities
    /// Returns true if any capability contains "pro", "enterprise", "raven", or "max"
    pub fn is_pro(&self) -> bool {
        self.capabilities.iter().any(|c| {
            c.contains("pro")
                || c.contains("enterprise")
                || c.contains("raven")
                || c.contains("max")
        })
    }

    /// Update cookie from the server response
    pub fn update_cookie_from_res(&mut self, res: &Response) {
        if let Some(s) = res.headers().get(SET_COOKIE).and_then(|h| h.to_str().ok()) {
            self.update_cookies(s)
        }
    }

    /// Update cookies from string
    fn update_cookies(&mut self, str: &str) {
        let str = str.split("\n").to_owned().collect::<Vec<_>>().join("");
        if str.is_empty() {
            return;
        }
        let re = RegexBuilder::new(r"^(path|expires|domain|HttpOnly|Secure|SameSite)[=;]*")
            .case_insensitive(true)
            .build()
            .unwrap();
        str.split(";")
            .filter(|s| !re.is_match(s) && !s.is_empty())
            .for_each(|s| {
                let Some((name, value)) = s.split_once("=").map(|(n, v)| (n.trim(), v.trim()))
                else {
                    return;
                };
                if name.is_empty() || value.is_empty() {
                    return;
                }
                self.cookies.insert(name.to_string(), value.to_string());
            });
    }

    /// Current cookie string that are used in requests
    fn header_cookie(&self) -> String {
        // check rotating guard
        self.cookies
            .iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ")
            .trim()
            .to_string()
    }

    /// Requests a new cookie from the cookie manager
    /// Updates the internal state with the new cookie and proxy configuration
    pub async fn request_cookie(&mut self) -> Result<(), ClewdrError> {
        let res = self.event_sender.request().await?;
        self.cookie = Some(res.clone());
        self.update_cookies(res.cookie.to_string().as_str());
        // load newest config
        self.proxy = CLEWDR_CONFIG.load().rquest_proxy.clone();
        self.endpoint = CLEWDR_CONFIG.load().endpoint();
        println!("Cookie: {}", res.cookie.to_string().green());
        Ok(())
    }

    /// Returns the current cookie to the cookie manager
    /// Optionally provides a reason for returning the cookie (e.g., invalid, banned)
    pub async fn return_cookie(&mut self, reason: Option<Reason>) {
        // return the cookie to the cookie manager
        if let Some(cookie) = self.cookie.take() {
            self.event_sender
                .return_cookie(cookie, reason)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send cookie: {}", e);
                });
        }
    }

    /// Deletes or renames the current chat conversation based on configuration
    /// If preserve_chats is true, the chat is renamed rather than deleted
    pub async fn clean_chat(&self) -> Result<(), ClewdrError> {
        let Some(ref org_uuid) = self.org_uuid else {
            return Ok(());
        };
        let Some(ref conv_uuid) = self.conv_uuid else {
            return Ok(());
        };
        // if preserve_chats is true, do not delete chat, just rename it
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}",
            self.endpoint, org_uuid, conv_uuid
        );
        if CLEWDR_CONFIG.load().preserve_chats {
            debug!("Renaming chat: {}", conv_uuid);
            let pld = json!({
                "name": format!("ClewdR-{}-{}", org_uuid, conv_uuid),
            });
            let _ = self
                .request(Method::PUT, endpoint)
                .json(&pld)
                .send()
                .await?;
            return Ok(());
        }
        debug!("Deleting chat: {}", conv_uuid);
        let _ = self.request(Method::DELETE, endpoint).send().await?;
        Ok(())
    }

    /// Upload images to the Claude.ai
    pub async fn upload_images(&self, imgs: Vec<ImageSource>) -> Vec<String> {
        // upload images
        let fut = imgs
            .into_iter()
            .map_while(|img| {
                // check if the image is base64
                if img.type_ != "base64" {
                    warn!("Image type is not base64");
                    return None;
                }
                // decode the image
                let bytes = BASE64_STANDARD
                    .decode(img.data.as_bytes())
                    .inspect_err(|e| {
                        warn!("Failed to decode image: {}", e);
                    })
                    .ok()?;
                // choose the file name based on the media type
                let file_name = match img.media_type.as_str() {
                    "image/png" => "image.png",
                    "image/jpeg" => "image.jpg",
                    "image/gif" => "image.gif",
                    "image/webp" => "image.webp",
                    "application/pdf" => "document.pdf",
                    _ => "file",
                };
                // create the part and form
                let part = Part::bytes(bytes).file_name(file_name);
                let form = Form::new().part("file", part);
                let endpoint = format!("{}/api/{}/upload", self.endpoint, self.org_uuid.as_ref()?);
                Some(
                    // send the request into future
                    self.request(Method::POST, endpoint)
                        .header_append("anthropic-client-platform", "web_claude_ai")
                        .multipart(form)
                        .send(),
                )
            })
            .collect::<Vec<_>>();

        // get upload responses
        let fut = join_all(fut)
            .await
            .into_iter()
            .map_while(|r| {
                // check if the response is ok
                r.inspect_err(|e| {
                    warn!("Failed to upload image: {}", e);
                })
                .ok()
            })
            .map(|r| async {
                // get the response json
                // extract the file_uuid
                let json = r
                    .json::<Value>()
                    .await
                    .inspect_err(|e| {
                        warn!("Failed to parse image response: {}", e);
                    })
                    .ok()?;
                Some(json["file_uuid"].as_str()?.to_string())
            })
            .collect::<Vec<_>>();

        // collect the results
        join_all(fut)
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    }
}
