# Provider economics

Prices read on the official pages while working. They change: re-check before any decision.

## Hosting and compute

| Provider | Item | Price |
|---|---|---|
| Cloudflare Workers Paid | plan | $5/month, 10 M requests included, then $0.30/M |
| Cloudflare | static assets | free |
| Cloudflare R2 Standard | storage | $0.015/GB-month; class A $4.50/M; class B $0.36/M; free egress |
| Cloudflare R2 Infrequent Access | storage | $0.01/GB; class A $9/M; class B $0.90/M |
| Cloudflare Workers AI | inference | $0.011 per 1,000 neurons; gpt-oss-120b $0.35 input / $0.75 output per M tokens |
| Cloudflare AI Gateway | gateway | free; 5% on unified-billing credits |
| Vercel | Hobby / Pro | $0 / $20 per seat |
| Vercel AI Gateway | gateway | no markup |

## Models

| Channel | Fees | Examples ($ per M tokens, input / output) |
|---|---|---|
| OpenRouter | 5.5% on credit purchases; BYOK free up to $25k/month then 5% | gpt-5.6-sol 4.40 / 22; gpt-oss-120b 0.15 / 0.60 |
| opencode Zen | 4.6% + $0.31 per purchase; Go $10, Go Plus $40, Teams $20 per member | |
| OpenAI direct | Batch and Flex -50% | gpt-5.6-sol 4 / 20; terra 2 / 12; luna 0.20 / 1.20 |
| Vercel AI Gateway | provider price | gpt-5.6-sol 4 / 20; gpt-oss-120b 0.10 / 0.50 |

## Levers

- Class A operations dominate a daily-backup bucket synced with `--checksum`; `--size-only` cuts them.
- A router that prefers free offers, then the cheapest paid ones, captures the largest saving; gateway margins (OpenRouter 5.5%, Zen 4.6%) disappear when calling providers directly or through Vercel AI Gateway.
