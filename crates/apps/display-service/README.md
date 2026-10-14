# Display service

The display service exposes crawler data over GraphQL. The crawler service
provides catalog, chapter, and crawl-job data to it over protobuf/gRPC.

## Run

Start the crawler with its normal database and crawl settings. Its gRPC API
listens on `0.0.0.0:50051` by default (`GRPC_ADDR` changes the address).

In another terminal:

```sh
cargo run -p display_service
```

The GraphQL endpoint is `http://127.0.0.1:3001/graphql`; the GraphiQL UI is at
`/playground`. Set `CRAWLER_GRPC_URL` to point to the crawler if it is not at
`http://127.0.0.1:50051`. `DISPLAY_ADDR` changes the GraphQL bind address.

## Frontend

```sh
cd ui/apps/display
npm install
npm run dev
```

Vite serves the React dashboard at `http://localhost:5173` and proxies its
GraphQL requests to the display service. The dashboard lists comics, chapters,
and the latest crawl jobs, refreshing activity every 10 seconds.
