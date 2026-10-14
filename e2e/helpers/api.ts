import type { APIRequestContext } from "@playwright/test";

type GraphQLResponse<T> = {
  data?: T;
  errors?: { message: string }[];
};

export async function queryGraphQL<T>(request: APIRequestContext, query: string) {
  const response = await request.post("/graphql", { data: { query } });
  if (!response.ok()) {
    throw new Error(`GraphQL request failed (${response.status()})`);
  }

  const payload = (await response.json()) as GraphQLResponse<T>;
  if (payload.errors?.length) {
    throw new Error(payload.errors.map(({ message }) => message).join("; "));
  }
  if (!payload.data) {
    throw new Error("GraphQL response is missing data");
  }
  return payload.data;
}

export function requiredE2EEnv(name: "E2E_ADMIN_KEY" | "E2E_CRAWL_KEY") {
  const value = process.env[name];
  if (!value) throw new Error(`Missing required environment variable: ${name}`);
  return value;
}
