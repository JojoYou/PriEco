use prieco_benchmark as benchmark;

#[tokio::main]
async fn main() {
    benchmark::run().await;
}
