import { expect, type Page } from "@playwright/test";

export class DashboardPage {
  constructor(private readonly page: Page) {}

  async open() {
    await this.page.goto("/");
  }

  async expectCatalog() {
    await expect(this.page.getByRole("heading", { name: "The library" })).toBeVisible();
    await expect(this.page.getByRole("heading", { name: "Crawl jobs" })).toBeVisible();
    await expect(this.page.getByRole("heading", { name: "E2E Test Comic" })).toBeVisible();
    await expect(this.page.getByText("e2e-library")).toBeVisible();
  }

  async viewChapters() {
    await this.page.getByRole("button", { name: /View chapters/ }).click();
    await expect(this.page.getByText("Chapter 1")).toBeVisible();
  }
}
