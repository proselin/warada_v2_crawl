use async_graphql::{Context, Error, Object, Result};
use crawler_proto::crawler_client::CrawlerClient;
use tonic::transport::Channel;

pub struct Query;

#[Object]
impl Query {
    async fn comics(&self, context: &Context<'_>) -> Result<Vec<Comic>> {
        let mut client = crawler_client(context)?;
        let response = client
            .list_comics(crawler_proto::Empty {})
            .await
            .map_err(graphql_error)?
            .into_inner();
        Ok(response.comics.into_iter().map(Comic::from).collect())
    }

    async fn comic(&self, context: &Context<'_>, slug: String) -> Result<Option<Comic>> {
        let mut client = crawler_client(context)?;
        match client.get_comic(crawler_proto::ComicRequest { slug }).await {
            Ok(response) => Ok(response.into_inner().comic.map(Comic::from)),
            Err(status) if status.code() == tonic::Code::NotFound => Ok(None),
            Err(status) => Err(graphql_error(status)),
        }
    }

    async fn chapters(&self, context: &Context<'_>, comic_slug: String) -> Result<Vec<Chapter>> {
        let mut client = crawler_client(context)?;
        let response = client
            .list_chapters(crawler_proto::ComicRequest { slug: comic_slug })
            .await
            .map_err(graphql_error)?
            .into_inner();
        Ok(response.chapters.into_iter().map(Chapter::from).collect())
    }

    async fn crawl_jobs(&self, context: &Context<'_>) -> Result<Vec<CrawlJob>> {
        let mut client = crawler_client(context)?;
        let response = client
            .list_crawl_jobs(crawler_proto::Empty {})
            .await
            .map_err(graphql_error)?
            .into_inner();
        Ok(response.jobs.into_iter().map(CrawlJob::from).collect())
    }
}

fn crawler_client(context: &Context<'_>) -> Result<CrawlerClient<Channel>> {
    context
        .data::<CrawlerClient<Channel>>()
        .cloned()
        .map_err(|_| Error::new("Crawler gRPC client is unavailable"))
}

fn graphql_error(status: tonic::Status) -> Error {
    Error::new(format!(
        "Crawler RPC failed ({:?}): {}",
        status.code(),
        status.message()
    ))
}

#[derive(async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct Comic {
    id: String,
    slug: String,
    title: String,
    author: Option<String>,
    description: Option<String>,
    status: String,
    chapter_count: Option<i32>,
    crawling_status: String,
}

impl From<crawler_proto::Comic> for Comic {
    fn from(comic: crawler_proto::Comic) -> Self {
        Self {
            id: comic.id.to_string(),
            slug: comic.slug,
            title: comic.title,
            author: comic.author,
            description: comic.description,
            status: comic.status,
            chapter_count: comic.chapter_count,
            crawling_status: comic.crawling_status,
        }
    }
}

#[derive(async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct Chapter {
    id: String,
    comic_id: String,
    chapter_number: String,
    position: i32,
    crawling_status: String,
    origin_url: String,
}

impl From<crawler_proto::Chapter> for Chapter {
    fn from(chapter: crawler_proto::Chapter) -> Self {
        Self {
            id: chapter.id.to_string(),
            comic_id: chapter.comic_id.to_string(),
            chapter_number: chapter.chapter_number,
            position: chapter.position,
            crawling_status: chapter.crawling_status,
            origin_url: chapter.origin_url,
        }
    }
}

#[derive(async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CrawlJob {
    id: String,
    slug: String,
    job_type: String,
    status: String,
    error: Option<String>,
}

impl From<crawler_proto::CrawlJob> for CrawlJob {
    fn from(job: crawler_proto::CrawlJob) -> Self {
        Self {
            id: job.id.to_string(),
            slug: job.slug,
            job_type: job.job_type,
            status: job.status,
            error: job.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{EmptyMutation, EmptySubscription, Schema};

    #[test]
    fn schema_exposes_catalog_and_crawl_activity() {
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
        let sdl = schema.sdl();

        for field in [
            "comics: [Comic!]!",
            "comic(slug: String!): Comic",
            "chapters(comicSlug: String!): [Chapter!]!",
            "crawlJobs: [CrawlJob!]!",
            "chapterCount: Int",
            "crawlingStatus: String!",
        ] {
            assert!(sdl.contains(field), "missing GraphQL field: {field}");
        }
    }
}
