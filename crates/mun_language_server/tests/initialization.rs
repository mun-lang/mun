mod support;

#[async_std::test]
async fn test_server() {
    let _server = support::Project::with_fixture("").server();
}
