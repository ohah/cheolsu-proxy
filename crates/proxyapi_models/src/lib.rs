use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxiedRequest {
    #[serde(with = "http_serde::method")]
    method: Method,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
    time: i64,
}

impl ProxiedRequest {
    pub fn new(
        method: Method,
        uri: Uri,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
        time: i64,
    ) -> Self {
        Self {
            method,
            uri,
            version,
            headers,
            body,
            time,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn time(&self) -> i64 {
        self.time
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxiedResponse {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
    time: i64,
}

impl ProxiedResponse {
    pub fn new(
        status: StatusCode,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
        time: i64,
    ) -> Self {
        Self {
            status,
            version,
            headers,
            body,
            time,
        }
    }

    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn time(&self) -> i64 {
        self.time
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestInfo(pub Option<ProxiedRequest>, pub Option<ProxiedResponse>);
