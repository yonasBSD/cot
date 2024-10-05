use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use indexmap::IndexMap;
use tower_sessions::Session;

use crate::db::Database;
use crate::error::ErrorRepr;
use crate::headers::FORM_CONTENT_TYPE;
use crate::router::Router;
use crate::{Body, Result};

pub type Request = http::Request<Body>;

#[async_trait]
pub trait RequestExt {
    #[must_use]
    fn project_config(&self) -> &crate::config::ProjectConfig;

    #[must_use]
    fn router(&self) -> &Router;

    #[must_use]
    fn path_params(&self) -> &PathParams;

    #[must_use]
    fn path_params_mut(&mut self) -> &mut PathParams;

    #[must_use]
    fn db(&self) -> &Database;

    #[must_use]
    fn session(&self) -> &Session;

    #[must_use]
    fn session_mut(&mut self) -> &mut Session;

    /// Get the request body as bytes. If the request method is GET or HEAD, the
    /// query string is returned. Otherwise, if the request content type is
    /// `application/x-www-form-urlencoded`, then the body is read and returned.
    /// Otherwise, an error is thrown.
    ///
    /// # Errors
    ///
    /// Throws an error if the request method is not GET or HEAD and the content
    /// type is not `application/x-www-form-urlencoded`.
    /// Throws an error if the request body could not be read.
    ///
    /// # Returns
    ///
    /// The request body as bytes.
    async fn form_data(&mut self) -> Result<Bytes>;

    #[must_use]
    fn content_type(&self) -> Option<&http::HeaderValue>;

    fn expect_content_type(&mut self, expected: &'static str) -> Result<()>;
}

#[async_trait]
impl RequestExt for Request {
    fn project_config(&self) -> &crate::config::ProjectConfig {
        self.extensions()
            .get::<Arc<crate::config::ProjectConfig>>()
            .expect("ProjectConfig extension missing")
    }

    fn router(&self) -> &Router {
        self.extensions()
            .get::<Arc<Router>>()
            .expect("Router extension missing")
    }

    fn path_params(&self) -> &PathParams {
        self.extensions()
            .get::<PathParams>()
            .expect("PathParams extension missing")
    }

    fn path_params_mut(&mut self) -> &mut PathParams {
        self.extensions_mut().get_or_insert_default::<PathParams>()
    }

    fn db(&self) -> &Database {
        self.extensions()
            .get::<Arc<Database>>()
            .expect("Database extension missing")
    }

    fn session(&self) -> &Session {
        self.extensions()
            .get::<Session>()
            .expect("Session extension missing. Did you forget to add the SessionMiddleware?")
    }

    fn session_mut(&mut self) -> &mut Session {
        self.extensions_mut()
            .get_mut::<Session>()
            .expect("Session extension missing. Did you forget to add the SessionMiddleware?")
    }

    async fn form_data(&mut self) -> Result<Bytes> {
        if self.method() == http::Method::GET || self.method() == http::Method::HEAD {
            if let Some(query) = self.uri().query() {
                return Ok(Bytes::copy_from_slice(query.as_bytes()));
            }

            Ok(Bytes::new())
        } else {
            self.expect_content_type(FORM_CONTENT_TYPE)?;

            let body = std::mem::take(self.body_mut());
            let bytes = body.into_bytes().await?;

            Ok(bytes)
        }
    }

    fn content_type(&self) -> Option<&http::HeaderValue> {
        self.headers().get(http::header::CONTENT_TYPE)
    }

    fn expect_content_type(&mut self, expected: &'static str) -> Result<()> {
        let content_type = self
            .content_type()
            .map_or("".into(), |value| String::from_utf8_lossy(value.as_bytes()));
        if content_type == expected {
            Ok(())
        } else {
            Err(ErrorRepr::InvalidContentType {
                expected,
                actual: content_type.into_owned(),
            }
            .into())
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathParams {
    params: IndexMap<String, String>,
}

impl Default for PathParams {
    fn default() -> Self {
        Self::new()
    }
}

impl PathParams {
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: IndexMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, value: String) {
        self.params.insert(name, value);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }
}

pub(crate) fn query_pairs(bytes: &Bytes) -> impl Iterator<Item = (Cow<str>, Cow<str>)> {
    form_urlencoded::parse(bytes.as_ref())
}
