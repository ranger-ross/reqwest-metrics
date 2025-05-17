use metrics_exporter_prometheus::PrometheusBuilder;
use reqwest_metrics::MetricsMiddleware;
use reqwest_middleware::ClientBuilder;

#[tokio::main]
async fn main() {
    // Register a metrics exporter.
    // In this case we will just expose a Prometheus metrics endpoint on localhost:9000/metrics
    //
    // You can change this to another exporter based on your needs.
    // See https://github.com/metrics-rs/metrics for more info.
    let handle = PrometheusBuilder::new().install_recorder().unwrap();

    // Build a reqwest client wrapped with `MetricsMiddleware`
    // Customizing the label attributes with the `MetricsMiddlewareBuilder`
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(
            MetricsMiddleware::builder()
                .method_label("http_request_method")
                .status_label("http_response_status")
                .host_label("http_request_host")
                .build(),
        )
        .build();

    // Send a request so we create some metrics.
    let _ = client.get("https://www.rust-lang.org").send().await;

    // Print the metrics in prometheus format
    println!("{}", handle.render());
}
