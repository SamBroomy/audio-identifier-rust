use wiremock::{
    Mock, ResponseTemplate,
    matchers::{any, method, path},
};

use crate::helpers::{ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_returns_422_for_invalid_data() {
    let app = TestApp::spawn_app().await;

    let test_cases = vec![
        (
            serde_json::json!({"title": "Newsletter title"}),
            "missing content",
        ),
        (
            serde_json::json!({"content": {"text": "Newsletter body as plain text", "html": "<p>Newsletter body as HTML</p>"}}),
            "missing title",
        ),
        (serde_json::json!({}), "missing title and content"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletter(invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            422,
            "The API did not return a 422 for {}",
            error_message
        );
    }
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = TestApp::spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = app.post_newsletter(newsletter_request_body).await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = TestApp::spawn_app().await;
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    app.get_confirmation_link(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;

    let response = reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(response.status(), 200);
}
