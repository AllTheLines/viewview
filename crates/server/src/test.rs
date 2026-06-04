#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt as _; // for `oneshot`

    async fn app() -> axum::Router {
        let config = crate::config::Config {
            db_dir: "./fixtures/shards".into(),
        };

        crate::app::build(config).await.unwrap()
    }

    #[tokio::test]
    async fn hello() {
        let response = app()
            .await
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), 100).await.unwrap();

        assert_eq!(body, "hello");
    }

    // TODO: Test parsing GeoJSON viewshed
}
