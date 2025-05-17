# reqwest-metrics

[![CI Status](https://github.com/ranger-ross/reqwest-metrics/workflows/Test/badge.svg)](https://github.com/ranger-ross/reqwest-metrics/actions)
[![docs.rs](https://docs.rs/reqwest-metrics/badge.svg)](https://docs.rs/reqwest-metrics)
[![crates.io](https://img.shields.io/crates/v/reqwest-metrics.svg)](https://crates.io/crates/reqwest-metrics)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ranger-ross/reqwest-metrics/blob/master/LICENSE)

[Metrics.rs](https://metrics.rs/) integration for [reqwest](https://docs.rs/reqwest/latest/reqwest/) using [reqwest-middleware](https://docs.rs/reqwest-middleware/latest/reqwest_middleware/)

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
            .client_name_label("custom_client_name")
            .method_label("http_request_method")
            .status_label("http_response_status")
            .build(),
    )
    .build();
```

Full list of labels:
* `client_name`
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

