/*!
[Metrics.rs](https://docs.rs/metrics/latest/metrics/) integration for [reqwest](https://docs.rs/reqwest/latest/reqwest/) using [reqwest-middleware](https://docs.rs/reqwest-middleware/latest/reqwest_middleware/)

## Features

* Adheres to [Open Telemetry HTTP Client Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#http-client)
* Customizable labels

# Usage

```rust
# use reqwest_middleware::ClientBuilder;
# use reqwest_metrics::MetricsMiddleware;
let client = ClientBuilder::new(reqwest::Client::new())
    .with(MetricsMiddleware::new())
    .build();
```

## Configuration

### Overriding label names

```rust
# use reqwest_middleware::ClientBuilder;
# use reqwest_metrics::MetricsMiddleware;
let client = ClientBuilder::new(reqwest::Client::new())
    .with(
        MetricsMiddleware::builder()
            .http_request_method_label("method")
            .http_response_status_label("status")
            .server_address_label("host")
            .build(),
    )
    .build();
```

Supported metrics:
* [`http.client.request.duration`](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#metric-httpclientrequestduration)
* [`http.client.request.body.size`](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#metric-httpclientrequestbodysize)
* [`http.client.response.body.size`](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#metric-httpclientresponsebodysize)

Supported labels:
* `http_request_method`
* `server_address`
* `server_port`
* `error_type`
* `http_response_status_code`
* `network_protocol_name`
* `network_protocol_version`
* `url_scheme`

## Motivation

This crate is heavily inspired by the [HTTP Client metrics](https://docs.spring.io/spring-boot/reference/actuator/metrics.html#actuator.metrics.supported.http-clients) provided by Spring. This crate aims to provide the same functionality while adhereing to Otel semantic conventions.


*/

#![deny(missing_docs)]

use std::{borrow::Cow, time::Instant};

use http::{Extensions, Method};
use metrics::{describe_histogram, histogram, Unit};
use reqwest_middleware::{
    reqwest::{Request, Response},
    Error, Middleware, Next, Result,
};

// Defaults should follow Open Telemetry when possible
// https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#http-client
const HTTP_CLIENT_REQUEST_DURATION: &str = "http.client.request.duration";
const HTTP_CLIENT_REQUEST_BODY_SIZE: &str = "http.client.request.body.size";
const HTTP_CLIENT_RESPONSE_BODY_SIZE: &str = "http.client.response.body.size";
// Labels
const HTTP_REQUEST_METHOD: &str = "http.request.method";
const SERVER_ADDRESS: &str = "server.address";
const SERVER_PORT: &str = "server.port";
const ERROR_TYPE: &str = "error.type";
const HTTP_RESPONSE_STATUS_CODE: &str = "http.response.status_code";
const NETWORK_PROTOCOL_NAME: &str = "network.protocol.name";
const NETWORK_PROTOCOL_VERSION: &str = "network.protocol.version";
const URL_SCHEME: &str = "url.scheme";

/// Middleware to handle emitting HTTP metrics for a reqwest client
/// NOTE: Creating a `[MetricMiddleware]` will describe a histogram on construction.
#[derive(Debug, Clone)]
pub struct MetricsMiddleware {
    label_names: LabelNames,
}

impl MetricsMiddleware {
    /// Create a new [`MetricsMiddleware`] with default labels.
    pub fn new() -> Self {
        Self::new_inner(LabelNames::default())
    }

    fn new_inner(label_names: LabelNames) -> Self {
        describe_histogram!(
            HTTP_CLIENT_REQUEST_DURATION,
            Unit::Seconds,
            "Duration of HTTP client requests."
        );
        describe_histogram!(
            HTTP_CLIENT_REQUEST_BODY_SIZE,
            Unit::Bytes,
            "Size of HTTP client request bodies."
        );
        describe_histogram!(
            HTTP_CLIENT_RESPONSE_BODY_SIZE,
            Unit::Bytes,
            "Size of HTTP client response bodies."
        );
        Self { label_names }
    }

    /// Create a new [`MetricsMiddlewareBuilder`] to create a customized [`MetricsMiddleware`]
    pub fn builder() -> MetricsMiddlewareBuilder {
        MetricsMiddlewareBuilder::new()
    }
}

#[derive(Debug, Clone)]
struct LabelNames {
    http_request_method: String,
    server_address: String,
    server_port: String,
    error_type: String,
    http_response_status: String,
    network_protocol_name: String,
    network_protocol_version: String,
    url_scheme: String,
}

impl Default for LabelNames {
    fn default() -> Self {
        Self {
            http_request_method: HTTP_REQUEST_METHOD.to_string(),
            server_address: SERVER_ADDRESS.to_string(),
            server_port: SERVER_PORT.to_string(),
            error_type: ERROR_TYPE.to_string(),
            http_response_status: HTTP_RESPONSE_STATUS_CODE.to_string(),
            network_protocol_name: NETWORK_PROTOCOL_NAME.to_string(),
            network_protocol_version: NETWORK_PROTOCOL_VERSION.to_string(),
            url_scheme: URL_SCHEME.to_string(),
        }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for [`MetricsMiddleware`]
#[derive(Debug, Clone)]
pub struct MetricsMiddlewareBuilder {
    label_names: LabelNames,
}

macro_rules! label_setters {
    // Match one or more method definitions
    (
        $(
            // For each definition, capture the method name, field name, and doc comment
            $(#[$attr:meta])*
            $method_name:ident, $field_name:ident
        );+
        $(;)?
    ) => {
        $(
            $(#[$attr])*
            pub fn $method_name<T: Into<String>>(&mut self, label: T) -> &mut Self {
                self.label_names.$field_name = label.into();
                self
            }
        )+
    };
}
impl MetricsMiddlewareBuilder {
    /// Create a new [`MetricsMiddlewareBuilder`]
    pub fn new() -> Self {
        Self {
            label_names: LabelNames::default(),
        }
    }

    label_setters! {
        /// Rename the `http.request.method` label.
        http_request_method_label, http_request_method;
        /// Rename the `server.address` label.
        server_address_label, server_address;
        /// Rename the `server.port` label.
        server_port_label, server_port;
        /// Rename the `error.type` label.
        error_type_label, error_type;
        /// Rename the `http.response.status` label.
        http_response_status_label, http_response_status;
        /// Rename the `network.protocol.name` label.
        network_protocol_name_label, network_protocol_name;
        /// Rename the `network.protocol.version` label.
        network_protocol_version_label, network_protocol_name;
        /// Rename the `url.scheme` label.
        url_scheme_label, url_scheme
    }

    /// Builds a [`MetricsMiddleware`]
    pub fn build(&self) -> MetricsMiddleware {
        MetricsMiddleware::new_inner(self.label_names.clone())
    }
}

impl Default for MetricsMiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Middleware for MetricsMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let http_request_method = http_request_method(&req);
        let url_scheme = url_scheme(&req);
        let server_address = server_address(&req);
        let server_port = server_port(&req);
        let network_protocol_version = network_protocol_version(&req);
        let request_body_size = req
            .body()
            .and_then(|body| body.as_bytes())
            .map(|bytes| bytes.len())
            .unwrap_or(0);

        let start = Instant::now();
        let res = next.run(req, extensions).await;
        let duration = start.elapsed();

        let mut labels = vec![
            (
                self.label_names.http_request_method.to_string(),
                http_request_method,
            ),
            (self.label_names.url_scheme.to_string(), url_scheme),
            (
                self.label_names.network_protocol_name.to_string(),
                Cow::Borrowed("http"),
            ),
        ];

        if let Some(server_address) = server_address {
            labels.push((
                self.label_names.server_address.to_string(),
                Cow::Owned(server_address),
            ));
        }

        if let Some(port) = server_port {
            labels.push((
                self.label_names.server_port.to_string(),
                Cow::Owned(port.to_string()),
            ));
        }

        if let Some(network_protocol_version) = network_protocol_version {
            labels.push((
                self.label_names.network_protocol_version.to_string(),
                Cow::Borrowed(network_protocol_version),
            ));
        }

        if let Some(status) = http_response_status(&res) {
            labels.push((self.label_names.http_response_status.to_string(), status));
        }

        if let Some(error) = error_type(&res) {
            labels.push((self.label_names.error_type.to_string(), error));
        }

        histogram!(HTTP_CLIENT_REQUEST_DURATION, &labels)
            .record(duration.as_millis() as f64 / 1000.0);

        histogram!(HTTP_CLIENT_REQUEST_BODY_SIZE, &labels).record(request_body_size as f64);

        // NOTE: The response body size is not *guaranteed* to be in the content-length header, but
        //       it will be added in nearly all modern HTTP implementations and waiting on the
        //       response body would be a fairly large performance pentality to force on our users.
        let response_body_size = res
            .as_ref()
            .ok()
            .and_then(|res| res.content_length())
            .unwrap_or(0);
        histogram!(HTTP_CLIENT_RESPONSE_BODY_SIZE, &labels).record(response_body_size as f64);

        res
    }
}

fn http_request_method(req: &Request) -> Cow<'static, str> {
    match req.method() {
        &Method::GET => Cow::Borrowed("GET"),
        &Method::POST => Cow::Borrowed("POST"),
        &Method::PUT => Cow::Borrowed("PUT"),
        &Method::DELETE => Cow::Borrowed("DELETE"),
        &Method::HEAD => Cow::Borrowed("HEAD"),
        &Method::OPTIONS => Cow::Borrowed("OPTIONS"),
        &Method::CONNECT => Cow::Borrowed("CONNECT"),
        &Method::PATCH => Cow::Borrowed("PATCH"),
        &Method::TRACE => Cow::Borrowed("TRACE"),
        method => Cow::Owned(method.as_str().to_string()),
    }
}

fn url_scheme(req: &Request) -> Cow<'static, str> {
    match req.url().scheme() {
        "http" => Cow::Borrowed("http"),
        "https" => Cow::Borrowed("https"),
        s => Cow::Owned(s.to_string()),
    }
}

fn server_address(req: &Request) -> Option<String> {
    req.url().host().map(|h| h.to_string())
}

fn server_port(req: &Request) -> Option<u16> {
    req.url().port_or_known_default()
}

fn http_response_status(res: &Result<Response>) -> Option<Cow<'static, str>> {
    res.as_ref()
        .map(|r| Cow::Owned(r.status().as_u16().to_string()))
        .ok()
}

fn error_type(res: &Result<Response>) -> Option<Cow<'static, str>> {
    Some(match res {
        Ok(res) if res.status().is_client_error() || res.status().is_server_error() => {
            Cow::Owned(res.status().as_str().to_string())
        }
        Err(Error::Middleware(err)) => Cow::Owned(format!("{err}")),
        Err(Error::Reqwest(err)) => Cow::Owned(format!("{err}")),
        _ => return None,
    })
}

#[cfg(target_arch = "wasm32")]
fn network_protocol_version(_req: &Request) -> Option<&'static str> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn network_protocol_version(req: &Request) -> Option<&'static str> {
    let version = req.version();

    Some(match version {
        http::Version::HTTP_09 => "0.9",
        http::Version::HTTP_10 => "1.0",
        http::Version::HTTP_11 => "1.1",
        http::Version::HTTP_2 => "2",
        http::Version::HTTP_3 => "3",
        _ => return None,
    })
}
