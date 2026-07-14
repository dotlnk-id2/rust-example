use goose::prelude::*;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_scenario(
            scenario!("ApiUser")
                .register_transaction(transaction!(api_get_users))
                .register_transaction(transaction!(api_create_user)),
        )
        // 设置全局默认值
        .set_default(GooseDefault::Host, "http://localhost:8088")?
        .set_default(GooseDefault::Users, 100)?
        .set_default(GooseDefault::RunTime, 30)?
        .execute()
        .await?;

    Ok(())
}

async fn api_get_users(user: &mut GooseUser) -> TransactionResult {
    // 带查询参数的 GET 请求
    let _response = user.get("/api/users?page=1&limit=10").await?;
    Ok(())
}

async fn api_create_user(user: &mut GooseUser) -> TransactionResult {
    // JSON POST 请求
    let payload = serde_json::json!({
        "name": "test_user",
        "email": "test@example.com"
    });
    let _response = user.post_json("/api/users", &payload).await?;
    Ok(())
}