use aws_config::BehaviorVersion;
use lambda_http::{service_fn, Error};
use obs_remote_lambda::do_;

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

    lambda_http::run(service_fn(|req| do_(&ssm, &logs, &ec2, "info", req))).await
}
