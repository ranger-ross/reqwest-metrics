use metrics_util::debugging::{DebuggingRecorder, Snapshotter};
use reqwest_metrics::{MetricsMiddleware, MetricsMiddlewareBuilder};
use reqwest_middleware::{reqwest, ClientBuilder};
use tokio::test;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const SNAPSHOT_FILTERS: [(&'static str, &'static str); 4] = [
    (
        r"Histogram\(\s*[\s\S]*?\s*\)",
        "Histogram([HISTOGRAM_VALUE])",
    ),
    (
        r#"Label\(\s*"server.port"\s*,\s*[\s\S]*?\s*\)"#,
        r#"Label("server.port", [PORT])"#,
    ),
    (
        r#"Label\(\s*"port"\s*,\s*[\s\S]*?\s*\)"#,
        r#"Label("port", [PORT])"#,
    ),
    (r#"hash: \d*"#, "hash: [HASH]"),
];

#[test]
async fn basic() {
    let snapshotter = install_debug_recorder();

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(MetricsMiddleware::new())
        .build();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let url = mock_server.uri();

    let res = client.get(format!("{url}/hello")).send().await.unwrap();
    assert_eq!(200, res.status().as_u16());

    let snapshot = snapshotter.snapshot();
    insta::with_settings!({filters => SNAPSHOT_FILTERS}, {
        insta::assert_debug_snapshot!(snapshot);
    });
}

#[test]
async fn custom_labels() {
    let snapshotter = install_debug_recorder();

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(
            MetricsMiddlewareBuilder::new()
                .http_request_method_label("method")
                .http_response_status_label("status")
                .server_address_label("host")
                .server_port_label("port")
                .network_protocol_name_label("protocol.name")
                .network_protocol_version_label("protocol.version")
                .url_scheme_label("scheme")
                .build(),
        )
        .build();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let url = mock_server.uri();

    let res = client.get(format!("{url}/hello")).send().await.unwrap();
    assert_eq!(200, res.status().as_u16());

    let snapshot = snapshotter.snapshot();
    insta::with_settings!({filters => SNAPSHOT_FILTERS}, {
        insta::assert_debug_snapshot!(snapshot);
    });
}

fn install_debug_recorder() -> Snapshotter {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    recorder.install().unwrap();
    snapshotter
}
