import { expect, test } from "bun:test";
import { fixture, recording, slim, type Gh } from "./lib/gh.ts";

test("slim keeps only the fields the tools read", () => {
  const raw = { full_name: "a/b", stargazers_count: 5, parent: { full_name: "c/d", owner: { login: "c", node_id: "x" }, topics: [] }, commits: [{ sha: "1", files: [] }] };
  expect(slim(raw)).toEqual({ full_name: "a/b", parent: { full_name: "c/d", owner: { login: "c" } } });
});

test("a recording replays in order and repeats the last response", async () => {
  const answers = [{ status: 200, json: { n: 1 } }, { status: 200, json: { n: 2 } }];
  const inner: Gh = { request: async () => answers.shift()! };
  const store = {};
  const rec = recording(inner, store);
  await rec.request("GET", "repos/a/b");
  await rec.request("GET", "repos/a/b");
  const replay = fixture(store);
  expect((await replay.request("GET", "repos/a/b")).json).toEqual({ n: 1 });
  expect((await replay.request("GET", "repos/a/b")).json).toEqual({ n: 2 });
  expect((await replay.request("GET", "repos/a/b")).json).toEqual({ n: 2 });
  await expect(replay.request("GET", "repos/x/y")).rejects.toThrow(/no response/);
});
