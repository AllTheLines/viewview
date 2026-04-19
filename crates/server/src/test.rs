#[cfg(test)]
mod tests {
    #![expect(clippy::indexing_slicing, reason = "These are just tests")]

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt as _; // for `oneshot`

    use std::convert::TryInto as _;
    use std::io::Read as _;

    fn read_u16_be(cursor: &mut std::io::Cursor<&[u8]>) -> u16 {
        let mut buffer = [0u8; 2];
        cursor.read_exact(&mut buffer).unwrap();
        u16::from_be_bytes(buffer)
    }

    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::integer_division,
        clippy::integer_division_remainder_used,
        reason = "These are just for tests"
    )]
    fn parse_payload_cursor(data: &[u8]) -> Vec<(u16, Vec<u16>)> {
        let mut cursor = std::io::Cursor::new(data);
        let mut out = Vec::new();

        while (cursor.position() as usize) < data.len() {
            let angle_id = read_u16_be(&mut cursor);

            let segments_length = read_u16_be(&mut cursor) as usize;
            let mut values = Vec::with_capacity(segments_length / 2);
            let mut buffer = vec![0u8; segments_length];
            cursor.read_exact(&mut buffer).unwrap();
            for chunk in buffer.chunks_exact(2) {
                values.push(u16::from_be_bytes(chunk.try_into().unwrap()));
            }
            out.push((angle_id, values));
        }

        out
    }

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

    #[tokio::test]
    async fn unpacking_viewshed() {
        let response = app()
            .await
            .oneshot(
                Request::get("/viewshed/-3.123,51.4898")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), 10000).await.unwrap();

        let segments = parse_payload_cursor(&body);

        assert_eq!(segments[0], (0, vec![0, 4]));
        assert_eq!(segments[300], (300, vec![0, 2, 3, 1]));
    }
}
