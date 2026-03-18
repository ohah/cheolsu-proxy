import type { HttpTransaction } from "@/entities/proxy";

import { jsonBody, tx } from "./mock-helpers";

/** GraphQL 트랜잭션 */
export const MOCK_GRAPHQL_TRANSACTIONS: HttpTransaction[] = [
  // 13. GraphQL query
  tx(
    {
      method: "POST",
      uri: "https://api.example.com/graphql",
      headers: { "content-type": "application/json" },
      ...jsonBody({
        query: "query { users(first: 10) { edges { node { id name email } } } }",
        variables: {},
      }),
    },
    {
      status: 200,
      ...jsonBody({
        data: {
          users: {
            edges: [
              { node: { id: "1", name: "Alice", email: "alice@example.com" } },
              { node: { id: "2", name: "Bob", email: "bob@example.com" } },
            ],
          },
        },
      }),
    },
  ),
];
