use goose::prelude::*;

use goose_eggs::{validate_and_load_static_assets, Validate};

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_scenario(
            scenario!("api_op_user")
                .register_transaction(transaction!(api_get_users).set_weight(2)?.set_name("获得用户 w=2"))
                .register_transaction(transaction!(api_create_user).set_weight(1)?.set_name("创建用户 w=1")),
        )
        // 设置全局默认值
        .set_default(GooseDefault::Host, "http://localhost:8088")?
        .set_default(GooseDefault::Users, 50)?
        .set_default(GooseDefault::HatchRate, "8")?
        .set_default(GooseDefault::RunTime, 100)?
        // .set_default(GooseDefault::CoordinatedOmissionMitigation,"Minimum")?
        .set_default(GooseDefault::ReportFile, "api_op_user-report.html")?
        .execute()
        .await?;

    Ok(())
}

async fn api_get_users(user: &mut GooseUser) -> TransactionResult {
    // 带查询参数的 GET 请求
    let _response = user.get("/api/users?page=1&limit=10").await?;

    let validate = &Validate::builder()
    .status(200)
    .text("hello get")
    .build();

    validate_and_load_static_assets(user, _response, &validate).await?;

    Ok(())
}

async fn api_create_user(user: &mut GooseUser) -> TransactionResult {
    // JSON POST 请求
    let payload = serde_json::json!({
        "name": "test_user",
        "email": "test@example.com"
    });
    let _response = user.post_json("/api/users", &payload).await?;

    let validate = &Validate::builder()
    .status(201)
    //.text("{\"name\": \"test_user\",\"email\": \"test@example.com\"}")
    .build();

    validate_and_load_static_assets(user, _response, &validate).await?;
    
    Ok(())
}

async fn check_availability(user: &mut GooseUser) -> TransactionResult {
    let mut response = user.get("/api/endpoint").await?;
    
    // Stop the test if server returns 503 Service Unavailable
    if let Ok(response) = response.response {
        if response.status() == 503 {
            goose::trigger_killswitch("Server returned 503: Service Unavailable");
        }
    }
    
    Ok(())
}
