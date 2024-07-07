use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_ec2::types::Filter;
use aws_sdk_ssm::types::{CloudWatchOutputConfig, Target};
use cwmc_obs_lambda::AppError;
use http::HeaderMap;
use lambda_http::{service_fn, Error, LambdaEvent};
use tokio::time::sleep;

async fn stop_streaming(ssm: &aws_sdk_ssm::Client, log: &aws_sdk_cloudwatchlogs::Client, ec2: &aws_sdk_ec2::Client, event: LambdaEvent<ApiGatewayProxyRequest>) -> Result<ApiGatewayProxyResponse, Error> {
    let params = event.payload.path_parameters;
    let proxy = params.get("proxy").ok_or(AppError(String::from("no path parameter named 'proxy'")))?;
    let wspw = params.get("wspw").ok_or(AppError(String::from("no path parameter named 'wspw'")))?;
    let port = event.payload.query_string_parameters.first("port").unwrap_or("9191");

    let output = ssm.send_command()
        .targets(Target::builder()
            .key("tag:Name")
            .set_values(Some(vec![String::from("cwmc")]))
            .build()
        )
        .document_name("AWS-RunShellScript")
        .parameters("commands", vec![format!("zrok access private {} --bind \"127.0.0.1:{}\" --headless &", proxy, port), format!("obs-cmd --websocket obsws://localhost:{}/{} streaming stop", port, wspw)])
        .parameters("executionTimeout", vec![String::from("3600")])
        .parameters("workingDirectory", vec![String::from("/home/ubuntu")])
        .cloud_watch_output_config(CloudWatchOutputConfig::builder()
            .cloud_watch_output_enabled(true)
            .build()
        )
        .send().await
        .map_err(|e| AppError(format!("Send Command Error: {:?}", e)))?;
    let command = output.command.ok_or(AppError(String::from("command does not exist")))?;
    let command_id = command.command_id().ok_or(AppError(String::from("command has no id")))?;

    sleep(Duration::from_secs(15)).await;

    let instance_output = ec2.describe_instances()
        .filters(Filter::builder()
            .name("tag:Name")
            .set_values(Some(vec![String::from("cwmc")]))
            .build()
        )
        .send().await
        .map_err(|e| AppError(format!("List Instances Error: {:?}", e)))?;

    let instance_id = instance_output.reservations()
        .get(0).ok_or(AppError(String::from("reservations list is empty")))?
        .instances()
        .get(0).ok_or(AppError(String::from("instances list is empty")))?
        .instance_id().ok_or(AppError(String::from("instance id does not exist")))?;

    let logs_output = log.get_log_events()
        .log_group_name("/aws/ssm/AWS-RunShellScript")
        .log_stream_name(format!("{}/{}/aws-runShellScript/stdout", command_id, instance_id))
        .send().await
        .map_err(|e| AppError(format!("Get Log Event Error: {:?}", e)))?;

    let events = logs_output.events.ok_or(AppError(String::from("logs does not exist")))?
        .into_iter().map(|e| e.message).collect::<Option<Vec<String>>>().ok_or(AppError(String::from("log message does not exist")))?;


    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HeaderMap::default(),
        multi_value_headers: HeaderMap::default(),
        body: Some(lambda_http::Body::Text(serde_json::to_string(&events).map_err(|e| AppError(format!("serialize error: {:?}", e)))?)),
        is_base64_encoded: false
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time() 
        .init();

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let ssm = aws_sdk_ssm::Client::new(&config);
    let logs = aws_sdk_cloudwatchlogs::Client::new(&config);
    let ec2 = aws_sdk_ec2::Client::new(&config);

    lambda_runtime::run(service_fn(|event: LambdaEvent<ApiGatewayProxyRequest>| stop_streaming(&ssm, &logs, &ec2, event))).await
}