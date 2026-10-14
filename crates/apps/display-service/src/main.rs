use std::error::Error;
use std::net::SocketAddr;

use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::routing::{get, post};
use crawler_proto::crawler_client::CrawlerClient;
use tonic::transport::Endpoint;

mod schema;

type DisplaySchema = Schema<schema::Query, EmptyMutation, EmptySubscription>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let crawler_url =
        std::env::var("CRAWLER_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let crawler = CrawlerClient::new(Endpoint::from_shared(crawler_url)?.connect_lazy());
    let schema = Schema::build(schema::Query, EmptyMutation, EmptySubscription)
        .data(crawler)
        .finish();

    let app = Router::new()
        .route("/graphql", post(graphql))
        .route("/playground", get(playground))
        .with_state(schema);
    let address: SocketAddr = std::env::var("DISPLAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Display GraphQL service listening on http://{address}/graphql");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn graphql(State(schema): State<DisplaySchema>, request: GraphQLRequest) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}

async fn playground() -> Html<String> {
    Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .finish(),
    )
}
