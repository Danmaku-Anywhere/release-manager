use std::time::Duration;

#[tokio::test]
#[ignore = "needs the network; run with cargo test -- --ignored"]
async fn client_built_like_the_app_completes_an_https_request() {
    release_manager_core::ensure_crypto_provider();
    let client = reqwest::Client::builder()
        .user_agent("release-manager-https-test")
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let response = client.get("https://api.github.com/zen").send().await.unwrap();

    assert!(response.status().is_success(), "status {}", response.status());
}
