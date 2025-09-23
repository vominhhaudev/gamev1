use common_net::telemetry;

fn main() {
    telemetry::init("services");
    tracing::info!("services runner started");
}
