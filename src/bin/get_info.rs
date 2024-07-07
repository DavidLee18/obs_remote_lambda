use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::ApiGatewayProxyRequest;
use obs_remote_lambda::do_;
use lambda_http::{service_fn, Error, LambdaEvent};

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

    lambda_runtime::run(service_fn(|event: LambdaEvent<ApiGatewayProxyRequest>| do_(&ssm, &logs, &ec2, "info", event))).await
}