# reqwest-metrics

[![CI Status](https://github.com/ranger-ross/reqwest-metrics/workflows/Test/badge.svg)](https://github.com/ranger-ross/reqwest-metrics/actions)
[![docs.rs](https://docs.rs/reqwest-metrics/badge.svg)](https://docs.rs/reqwest-metrics)
[![crates.io](https://img.shields.io/crates/v/reqwest-metrics.svg)](https://crates.io/crates/reqwest-metrics)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ranger-ross/reqwest-metrics/blob/master/LICENSE)

[Metrics.rs](https://metrics.rs/) integration for [reqwest](https://docs.rs/reqwest/latest/reqwest/) using [reqwest-middleware](https://docs.rs/reqwest-middleware/latest/reqwest_middleware/)

## Features 

* Adheres to [Open Telemetry HTTP Client Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#http-client)
* Customizable labels

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
            .http_request_method_label("method")
            .http_response_status_label("status")
            .server_address_label("host")
            .build(),
    )
    .build();
```

Full list of labels:
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

