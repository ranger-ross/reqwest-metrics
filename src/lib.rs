/*!
[Metrics.rs](https://docs.rs/metrics/latest/metrics/) integration for [reqwest](https://docs.rs/reqwest/latest/reqwest/) using [reqwest-middleware](https://docs.rs/reqwest-middleware/latest/reqwest_middleware/)

## Features

* `http_client_requests_seconds` metric (histogram)
* Customizable labels
* Ability enable/disable high cardinality meteric labels like `uri`

# Usage

```rust
let client = ClientBuilder::new(reqwest::Client::new())
    .with(MetricsMiddleware::new())
    .build();
```

## Configuration

### Overriding label names

```rust
let client = ClientBuilder::new(reqwest::Client::new())
    .with(
        MetricsMiddleware::builder()
            .method_label("http_request_method")
            .status_label("http_response_status")
            .host_label("http_request_host")
            .build(),
    )
    .build();
```

Full list of labels:
* `method`
* `outcome`
* `scheme`
* `host`
* `port`
* `status`
* `uri` (disabled by default)

### Enabling `uri` label


```rust
let client = ClientBuilder::new(reqwest::Client::new())
    .with(MetricsMiddleware::builder().enable_uri().build())
    .build();

```

## Motivation

This crate is heavily inspired by the [HTTP Client metrics](https://docs.spring.io/spring-boot/reference/actuator/metrics.html#actuator.metrics.supported.http-clients) provided by Spring. This crate aims to provide the same functionality (although some configuration is required)

*/

#![deny(missing_docs)]

use std::{borrow::Cow, time::Instant};

use http::{Extensions, Method};
use metrics::histogram;
use reqwest_middleware::{
    reqwest::{Request, Response},
    Middleware, Next, Result,
};

const METHOD: &str = "method";
const OUTCOME: &str = "outcome";
const SCHEME: &str = "scheme";
const HOST: &str = "host";
const PORT: &str = "port";
const STATUS: &str = "status";
const URI: &str = "uri";

/// Middleware to handle emitting HTTP metrics for a reqwest client
#[derive(Debug, Clone)]
pub struct MetricsMiddleware {
    enable_uri: bool,
    label_names: LabelNames,
}

impl MetricsMiddleware {
    /// Create a new [`MetricsMiddleware`] with default labels. (`uri` label is disabled by default)
    pub fn new() -> Self {
        Self {
            enable_uri: false,
            label_names: LabelNames::default(),
        }
    }

    /// Create a new [`MetricsMiddlewareBuilder`] to create a customized [`MetricsMiddleware`]
    pub fn builder() -> MetricsMiddlewareBuilder {
        MetricsMiddlewareBuilder::new()
    }
}

#[derive(Debug, Clone)]
struct LabelNames {
    method: String,
    outcome: String,
    scheme: String,
    host: String,
    port: String,
    status: String,
    uri: String,
}

impl Default for LabelNames {
    fn default() -> Self {
        Self {
            method: METHOD.to_string(),
            outcome: OUTCOME.to_string(),
            scheme: SCHEME.to_string(),
            host: HOST.to_string(),
            port: PORT.to_string(),
            status: STATUS.to_string(),
            uri: URI.to_string(),
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
    enable_uri: bool,
    label_names: LabelNames,
}

impl MetricsMiddlewareBuilder {
    /// Create a new [`MetricsMiddlewareBuilder`]
    pub fn new() -> Self {
        Self {
            enable_uri: false,
            label_names: LabelNames::default(),
        }
    }

    /// Rename the `method` label.
    pub fn method_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.method = label.into();
        self
    }

    /// Rename the `outcome` label.
    pub fn outcome_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.outcome = label.into();
        self
    }

    /// Rename the `scheme` label.
    pub fn scheme_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.scheme = label.into();
        self
    }

    /// Rename the `host` label.
    pub fn host_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.host = label.into();
        self
    }

    /// Rename the `port` label.
    pub fn port_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.port = label.into();
        self
    }

    /// Rename the `status` label.
    pub fn status_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.status = label.into();
        self
    }

    /// Rename the `uri` label.
    pub fn uri_label<T: Into<String>>(&mut self, label: T) -> &mut Self {
        self.label_names.uri = label.into();
        self
    }

    /// Enable `uri` label in metrics.
    ///
    /// <div class="warning"> WARNING: Enabling URIs can lead to high cardinality metrics.</div>
    pub fn enable_uri(&mut self) -> &mut Self {
        self.enable_uri = true;
        self
    }

    /// Builds a [`MetricsMiddleware`]
    pub fn build(&self) -> MetricsMiddleware {
        MetricsMiddleware {
            enable_uri: self.enable_uri,
            label_names: self.label_names.clone(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for MetricsMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let method = method(&req);
        let scheme = scheme(&req);
        let host = host(&req);
        let port = port(&req);
        let uri = uri(&req);

        let start = Instant::now();
        let res = next.run(req, extensions).await;
        let duration = start.elapsed();

        let outcome = outcome(&res);

        let mut labels = vec![
            (self.label_names.method.to_string(), method),
            (self.label_names.outcome.to_string(), Cow::Borrowed(outcome)),
            (self.label_names.scheme.to_string(), scheme),
        ];

        if let Some(host) = host {
            labels.push((self.label_names.host.to_string(), Cow::Owned(host)));
        }

        if let Some(port) = port {
            labels.push((
                self.label_names.port.to_string(),
                Cow::Owned(port.to_string()),
            ));
        }

        if let Some(status) = status(&res) {
            labels.push((self.label_names.status.to_string(), status));
        }

        if self.enable_uri {
            labels.push((self.label_names.uri.to_string(), Cow::Owned(uri)));
        }

        histogram!("http_client_requests_seconds", &labels)
            .record(duration.as_millis() as f64 / 1000.0);

        res
    }
}

fn method(req: &Request) -> Cow<'static, str> {
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

fn scheme(req: &Request) -> Cow<'static, str> {
    match req.url().scheme() {
        "http" => Cow::Borrowed("http"),
        "https" => Cow::Borrowed("https"),
        s => Cow::Owned(s.to_string()),
    }
}

fn uri(req: &Request) -> String {
    let path = req.url().path();
    return if let Some(query) = req.url().query() {
        format!("{path}?{query}")
    } else {
        path.to_string()
    };
}

fn host(req: &Request) -> Option<String> {
    req.url().host().map(|h| h.to_string())
}

fn port(req: &Request) -> Option<u16> {
    req.url().port_or_known_default()
}

fn status(res: &Result<Response>) -> Option<Cow<'static, str>> {
    return res
        .as_ref()
        .map(|r| Cow::Owned(r.status().as_u16().to_string()))
        .ok();
}

fn outcome(res: &Result<Response>) -> &'static str {
    let Ok(res) = res else {
        return "UNKNOWN";
    };
    return match res.status().as_u16() {
        100..200 => "INFORMATIONAL",
        200..300 => "SUCCESS",
        300..400 => "REDIRECTION",
        400..500 => "CLIENT_ERROR",
        500..600 => "INFORMATIONAL",
        _ => "UNKNOWN",
    };
}
