use tonic::{Request, Response, Status};

use comic_crawler::state::{AppState, ChapterRecord, ComicRecord};

#[derive(Clone)]
pub struct CrawlerGrpc {
    state: AppState,
}

impl CrawlerGrpc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl crawler_proto::crawler_server::Crawler for CrawlerGrpc {
    async fn list_comics(
        &self,
        _: Request<crawler_proto::Empty>,
    ) -> Result<Response<crawler_proto::ListComicsResponse>, Status> {
        let comics = if self.state.shared.database.is_enabled() {
            self.state
                .shared
                .database
                .list_comics_for_display()
                .await
                .map_err(Status::unavailable)?
        } else {
            self.state
                .shared
                .comics
                .read()
                .map_err(|_| Status::internal("Comic store lock is poisoned"))?
                .values()
                .cloned()
                .collect()
        };
        Ok(Response::new(crawler_proto::ListComicsResponse {
            comics: comics.into_iter().map(comic_to_proto).collect(),
        }))
    }

    async fn get_comic(
        &self,
        request: Request<crawler_proto::ComicRequest>,
    ) -> Result<Response<crawler_proto::ComicResponse>, Status> {
        let slug = request.into_inner().slug;
        let comic = if self.state.shared.database.is_enabled() {
            self.state
                .shared
                .database
                .get_comic_for_display(&slug)
                .await
                .map_err(Status::unavailable)?
        } else {
            self.state
                .shared
                .comics
                .read()
                .map_err(|_| Status::internal("Comic store lock is poisoned"))?
                .get(&slug)
                .cloned()
        };
        let comic = comic.ok_or_else(|| Status::not_found("Comic not found"))?;
        Ok(Response::new(crawler_proto::ComicResponse {
            comic: Some(comic_to_proto(comic)),
        }))
    }

    async fn list_chapters(
        &self,
        request: Request<crawler_proto::ComicRequest>,
    ) -> Result<Response<crawler_proto::ListChaptersResponse>, Status> {
        let slug = request.into_inner().slug;
        let chapters = if self.state.shared.database.is_enabled() {
            let comic = self
                .state
                .shared
                .database
                .get_comic_for_display(&slug)
                .await
                .map_err(Status::unavailable)?
                .ok_or_else(|| Status::not_found("Comic not found"))?;
            self.state
                .shared
                .database
                .list_chapters_for_display(comic.id)
                .await
                .map_err(Status::unavailable)?
        } else {
            let comic = self
                .state
                .shared
                .comics
                .read()
                .map_err(|_| Status::internal("Comic store lock is poisoned"))?
                .get(&slug)
                .cloned()
                .ok_or_else(|| Status::not_found("Comic not found"))?;
            self.state.get_chapters_for_comic(comic.id)
        };
        Ok(Response::new(crawler_proto::ListChaptersResponse {
            chapters: chapters.into_iter().map(chapter_to_proto).collect(),
        }))
    }

    async fn list_crawl_jobs(
        &self,
        _: Request<crawler_proto::Empty>,
    ) -> Result<Response<crawler_proto::ListCrawlJobsResponse>, Status> {
        let jobs = self
            .state
            .shared
            .database
            .list_crawl_jobs_for_display()
            .await
            .map_err(Status::unavailable)?;
        Ok(Response::new(crawler_proto::ListCrawlJobsResponse {
            jobs: jobs.into_iter().map(job_to_proto).collect(),
        }))
    }
}

fn comic_to_proto(comic: ComicRecord) -> crawler_proto::Comic {
    crawler_proto::Comic {
        id: comic.id,
        slug: comic.slug,
        title: comic.title,
        author: comic.author,
        description: comic.description,
        status: comic.status,
        chapter_count: comic.chapter_count,
        crawling_status: comic.crawling_status,
    }
}

fn chapter_to_proto(chapter: ChapterRecord) -> crawler_proto::Chapter {
    crawler_proto::Chapter {
        id: chapter.id,
        comic_id: chapter.comic_id,
        chapter_number: chapter.chapter_num,
        position: chapter.position,
        crawling_status: chapter.crawling_status,
        origin_url: chapter.origin_url,
    }
}

fn job_to_proto(job: comic_crawler::db::CrawlJob) -> crawler_proto::CrawlJob {
    crawler_proto::CrawlJob {
        id: job.id,
        slug: job.slug,
        job_type: job.job_type,
        status: job.status,
        error: job.error,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crawl_job_conversion_preserves_optional_error() {
        let job = super::job_to_proto(comic_crawler::db::CrawlJob {
            id: 7,
            slug: "comic".to_owned(),
            job_type: "retry".to_owned(),
            status: "failed".to_owned(),
            error: Some("network error".to_owned()),
        });

        assert_eq!(job.id, 7);
        assert_eq!(job.slug, "comic");
        assert_eq!(job.job_type, "retry");
        assert_eq!(job.status, "failed");
        assert_eq!(job.error.as_deref(), Some("network error"));
    }
}
