# AI_USAGE.md

## Which AI tools, and for what

Before opening any AI tool, I read the assignment doc myself and wrote my own notes on what it was actually asking for. Only after that did I hand the doc, along with my notes, to Claude and ChatGPT and ask each of them to point out anything in the assignment I might have missed or misread — a check on my own understanding, not a substitute for it.

From there, each tool had a specific job:

- **ChatGPT** — drafting the initial database schema and checking it afterward (constraints, indexes, normalization).
- **Antigravity, Claude, and GitHub Copilot** (all free versions) — writing the actual implementation code.
- **Claude and ChatGPT together** — writing the documentation (`DESIGN.md`, `README.md`, this file, `docs/openapi.yaml`).

I split the documentation work between the two deliberately: ChatGPT for the parts I wanted written in plain, simple language — I find it easier to follow, and after using it a lot it's tuned to the kind of response I actually prefer. Claude for the architecture and decision-making sections, because it reasons through trade-offs more thoroughly and I found it more efficient to work with there.


## Two decisions I made myself, independent of (or against) AI suggestion

**1. Five invoice states, including `uncollectible` as distinct from `void`.**
AI's first pass at this had actually merged the two into a single "cancelled" state. I caught that during code review and split them back apart myself, because they mean different things to a business reading its own books: `void` is "this was never really owed" (e.g. created by mistake), `uncollectible` is "this was legitimately billed and is now being written off" (e.g. after repeated failed payment attempts). Collapsing them into one state would have been simpler, but it throws away information a real accounting process cares about.

**2. Overall code structure.**
The project's shape — a `db/`, `handlers/`, `models/`, `routes/`, and `services/` module, one file per resource inside each — is something I laid out myself, not something I let an AI tool default to. I'm used to writing clean, layered code where each piece has exactly one job, so I kept that structure fixed through the build and had the AI tools write code to fit inside it, rather than let them decide the shape of the project.

## One thing AI got wrong

While writing the concurrency test this session, all 15 concurrent `POST /invoices/{id}/pay` requests came back `404` instead of the expected mix of `200`/`409`. The reason: the pay route was registered in its own separate file, `routes/payments.rs`, instead of alongside the rest of the invoice routes in `routes/invoices.rs`. In actix-web, once a request matches a scope's URL prefix, routing commits to that scope and won't fall through to check a different file just because its path looks similar. So every `/invoices/...` request got claimed by the `/invoices` scope, found no match for `/{id}/pay` inside it, and actix returned a plain 404 before the payments route was ever considered. In short, the core payment endpoint had never actually worked over real HTTP in this build — I confirmed the same 404 by hitting the running `docker compose` stack directly with `curl`, not just through the test. Fixed by moving the `/{id}/pay` route into the same scope as the rest of `/invoices` and deleting the now-unused `routes/payments.rs`, then re-checked it end-to-end with `curl` against the rebuilt Docker image.
