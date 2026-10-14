import { expect, test } from "@playwright/test";
import { queryGraphQL, requiredE2EEnv } from "../helpers/api";
import { DashboardPage } from "../page-objects/dashboard.page";

test("gateway serves the dashboard and both federation remotes", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const dashboard = new DashboardPage(page);

  const activityRemote = page.waitForResponse(
    (response) =>
      response.url().includes("/remotes/activity/remoteEntry.js") &&
      response.status() === 200,
  );
  const uiKitRemote = page.waitForResponse(
    (response) =>
      response.url().includes("/remotes/ui-kit/remoteEntry.js") &&
      response.status() === 200,
  );

  await dashboard.open();
  await dashboard.expectCatalog();
  await dashboard.viewChapters();
  await Promise.all([activityRemote, uiKitRemote]);
  expect(pageErrors).toEqual([]);
});

test("gateway connects crawler, GraphQL, and persisted auth token flows", async ({
  request,
}) => {
  const health = await request.get("/health");
  expect(health.ok()).toBeTruthy();
  expect(await health.text()).toBe("OK");

  const toolsHealth = await request.get("/tools/health");
  expect(toolsHealth.ok()).toBeTruthy();
  expect(await toolsHealth.text()).toBe("OK");

  const catalog = await queryGraphQL<{
    comics: { id: string; slug: string }[];
    chapters: { chapterNumber: string }[];
    crawlJobs: { slug: string; status: string }[];
  }>(request, `{
    comics { id slug }
    chapters(comicSlug: "e2e-library") { chapterNumber }
    crawlJobs { slug status }
  }`);
  expect(catalog.comics).toEqual([{ id: "900001", slug: "e2e-library" }]);
  expect(catalog.chapters).toEqual([{ chapterNumber: "1" }]);
  expect(catalog.crawlJobs).toEqual([{ slug: "e2e-library", status: "finished" }]);

  const crawlerDenied = await request.post("/api/v1/crawl/nettruyen/comic", {
    data: {},
  });
  expect(crawlerDenied.status()).toBe(401);

  const crawlerInvalid = await request.post("/api/v1/crawl/nettruyen/comic", {
    headers: { "x-crawl-api-key": requiredE2EEnv("E2E_CRAWL_KEY") },
    data: {},
  });
  expect(crawlerInvalid.status()).toBe(400);

  const adminDenied = await request.get("/v1/service-clients");
  expect(adminDenied.status()).toBe(401);

  const clientId = `e2e-${Date.now()}`;
  const createClient = await request.post("/v1/service-clients", {
    headers: { authorization: `Bearer ${requiredE2EEnv("E2E_ADMIN_KEY")}` },
    data: { client_id: clientId, scopes: ["api:read"] },
  });
  expect(createClient.status()).toBe(201);
  const client = await createClient.json();

  const tokenResponse = await request.post("/oauth/token", {
    form: {
      grant_type: "client_credentials",
      client_id: client.client_id,
      client_secret: client.client_secret,
      scope: "api:read",
    },
  });
  expect(tokenResponse.ok()).toBeTruthy();
  const token = await tokenResponse.json();
  expect(token.token_type).toBe("Bearer");

  const verified = await request.get("/v1/verify", {
    headers: { authorization: `Bearer ${token.access_token}` },
  });
  expect(verified.ok()).toBeTruthy();
  expect(await verified.json()).toEqual({
    subject: clientId,
    principal_type: "service",
    scopes: ["api:read"],
  });
});
