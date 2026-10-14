import { expect, test } from "@rstest/core";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Badge, Button, Card } from "../src/index.js";

test("shared components render shadcn-style variants and semantic card elements", () => {
  expect(renderToStaticMarkup(createElement(Badge, { variant: "running" }, "Running")))
    .toContain('class="ui-badge ui-badge--running"');
  expect(renderToStaticMarkup(createElement(Button, { variant: "outline" }, "Open")))
    .toContain('class="ui-button ui-button--outline"');
  expect(renderToStaticMarkup(createElement(Card, { as: "article" }, "Card")))
    .toContain('<article data-slot="card" class="ui-card">');
});
