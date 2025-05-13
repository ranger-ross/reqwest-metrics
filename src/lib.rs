use std::{borrow::Cow, time::SystemTime};

use http::{Extensions, Method};
use metrics::histogram;
use reqwest_middleware::{
    Middleware, Next, Result,
    reqwest::{Request, Response},
};

#[derive(Debug, Clone)]
pub struct MetricsMiddleware {
    enable_uri: bool,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self { enable_uri: false }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MetricsMiddlewareBuilder {
    enable_uri: bool,
}

impl MetricsMiddlewareBuilder {
    pub fn new() -> Self {
        Self { enable_uri: false }
    }

    pub fn enable_uri(&mut self) -> &mut Self {
        self.enable_uri = true;
        self
    }

    pub fn build(&self) -> MetricsMiddleware {
        MetricsMiddleware {
            enable_uri: self.enable_uri,
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
        let client_name = client_name(&req);
        let method = method(&req);
        let uri = uri(&req);

        let start = SystemTime::now();
        let res = next.run(req, extensions).await;
        let duration = SystemTime::now().duration_since(start).unwrap();

        let outcome = outcome(&res);

        let mut labels = vec![
            ("client_name", Cow::Owned(client_name)),
            ("method", method),
            ("outcome", Cow::Borrowed(outcome)),
        ];

        if let Some(status) = status(&res) {
            labels.push(("status", status));
        }

        if self.enable_uri {
            labels.push(("uri", Cow::Owned(uri)));
        }

        histogram!("http_client_requests_seconds", &labels)
            .record(duration.as_millis() as f64 / 1000.0);

        res
    }
}

fn client_name(req: &Request) -> String {
    return req.url().host_str().map(str::to_string).unwrap_or_default();
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

fn uri(req: &Request) -> String {
    let path = req.url().path();
    return if let Some(query) = req.url().query() {
        format!("{path}?{query}")
    } else {
        path.to_string()
    };
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
