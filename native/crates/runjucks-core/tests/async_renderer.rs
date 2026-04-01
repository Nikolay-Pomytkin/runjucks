//! Tests for the async renderer.

#[cfg(feature = "async")]
mod tests {
    use runjucks_core::Environment;
    use serde_json::json;
    use std::pin::Pin;
    use std::sync::Arc;

    /// Basic async render should produce the same output as sync render.
    #[tokio::test]
    async fn async_render_matches_sync() {
        let env = Environment::default();
        let tpl = "Hello, {{ name }}!".to_string();
        let ctx = json!({ "name": "World" });
        let sync_out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
        let async_out = env.render_string_async(tpl, ctx).await.unwrap();
        assert_eq!(sync_out, async_out);
    }

    /// Async render with arithmetic expressions.
    #[tokio::test]
    async fn async_render_arithmetic() {
        let env = Environment::default();
        let out = env
            .render_string_async("{{ 1 + 2 }}".to_string(), json!({}))
            .await
            .unwrap();
        assert_eq!(out, "3");
    }

    /// Async render with for loop.
    #[tokio::test]
    async fn async_render_for_loop() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% for x in items %}{{ x }}{% endfor %}".to_string(),
                json!({ "items": [1, 2, 3] }),
            )
            .await
            .unwrap();
        assert_eq!(out, "123");
    }

    /// Async render with if/else.
    #[tokio::test]
    async fn async_render_if_else() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% if show %}yes{% else %}no{% endif %}".to_string(),
                json!({ "show": true }),
            )
            .await
            .unwrap();
        assert_eq!(out, "yes");
    }

    /// Async custom filter.
    #[tokio::test]
    async fn async_custom_filter() {
        let mut env = Environment::default();
        env.add_async_filter(
            "shout".to_string(),
            Arc::new(|input: &serde_json::Value, _args: &[serde_json::Value]| {
                let s = input.as_str().unwrap_or("").to_uppercase();
                Box::pin(async move { Ok(serde_json::Value::String(s)) })
                    as Pin<
                        Box<
                            dyn std::future::Future<
                                    Output = runjucks_core::errors::Result<serde_json::Value>,
                                > + Send,
                        >,
                    >
            }),
        );
        let out = env
            .render_string_async("{{ name | shout }}".to_string(), json!({ "name": "hello" }))
            .await
            .unwrap();
        assert_eq!(out, "HELLO");
    }

    /// Async custom global function.
    #[tokio::test]
    async fn async_custom_global() {
        let mut env = Environment::default();
        env.add_async_global_callable(
            "fetchData".to_string(),
            Arc::new(
                |_args: &[serde_json::Value], _kwargs: &[(String, serde_json::Value)]| {
                    Box::pin(async { Ok(serde_json::Value::String("fetched!".into())) })
                        as Pin<
                            Box<
                                dyn std::future::Future<
                                        Output = runjucks_core::errors::Result<serde_json::Value>,
                                    > + Send,
                            >,
                        >
                },
            ),
        );
        let out = env
            .render_string_async("{{ fetchData() }}".to_string(), json!({}))
            .await
            .unwrap();
        assert_eq!(out, "fetched!");
    }

    /// asyncEach tag works in async mode.
    #[tokio::test]
    async fn async_each_tag() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% asyncEach item in items %}{{ item }}{% endeach %}".to_string(),
                json!({ "items": ["a", "b", "c"] }),
            )
            .await
            .unwrap();
        assert_eq!(out, "abc");
    }

    /// asyncAll tag works in async mode.
    #[tokio::test]
    async fn async_all_tag() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% asyncAll item in items %}{{ item }}{% endall %}".to_string(),
                json!({ "items": [1, 2, 3] }),
            )
            .await
            .unwrap();
        assert_eq!(out, "123");
    }

    /// ifAsync tag works in async mode.
    #[tokio::test]
    async fn if_async_tag() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% ifAsync show %}visible{% endif %}".to_string(),
                json!({ "show": true }),
            )
            .await
            .unwrap();
        assert_eq!(out, "visible");
    }

    /// Sync renderer should return error for async-only tags.
    #[test]
    fn sync_renderer_rejects_async_tags() {
        let env = Environment::default();
        let result = env.render_string(
            "{% asyncEach x in items %}{{ x }}{% endeach %}".to_string(),
            json!({ "items": [1] }),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("async render mode"));
    }

    /// Set tag in async mode.
    #[tokio::test]
    async fn async_set_tag() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{% set x = 42 %}{{ x }}".to_string(),
                json!({}),
            )
            .await
            .unwrap();
        assert_eq!(out, "42");
    }

    /// Filters chain in async mode.
    #[tokio::test]
    async fn async_filter_chain() {
        let env = Environment::default();
        let out = env
            .render_string_async(
                "{{ name | upper | length }}".to_string(),
                json!({ "name": "hello" }),
            )
            .await
            .unwrap();
        assert_eq!(out, "5");
    }
}
