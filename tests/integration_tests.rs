use metrics_util::debugging::{DebuggingRecorder, Snapshotter};
use reqwest_metrics::{MetricsMiddleware, MetricsMiddlewareBuilder};
use reqwest_middleware::{ClientBuilder, reqwest};
use tokio::test;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const SNAPSHOT_FILTERS: [(&'static str, &'static str); 4] = [
    (
        r"Histogram\(\s*[\s\S]*?\s*\)",
        "Histogram([HISTOGRAM_VALUE])",
    ),
    (
        r#"Label\(\s*"port"\s*,\s*[\s\S]*?\s*\)"#,
        r#"Label("port", [PORT])"#,
    ),
    (
        r#"Label\(\s*"my_port"\s*,\s*[\s\S]*?\s*\)"#,
        r#"Label("my_port", [PORT])"#,
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
                .client_name_label("my_host")
                .method_label("my_method")
                .status_label("my_status")
                .scheme_label("my_scheme")
                .outcome_label("my_outcome")
                .host_label("my_host")
                .port_label("my_port")
                .uri_label("my_uri")
                .enable_uri()
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
